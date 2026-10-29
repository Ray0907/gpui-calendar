//! The start and end days of the popover: picked from gpui-kit's date picker, never typed.

use chrono::NaiveTime;
use gpui_kit::component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_kit::{
    AnyElement, App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Subscription,
    TestSupportExt, Window, div, prelude::*, px,
};
use gpui_kit::base::actions::Confirm;

use super::Form;
use super::fields::{FIELD_H, FIELD_R, error_note, label};
use crate::domain::form::{Field, parse_date};
use crate::theme::resolve::Theme;
use crate::ui::controls::{c, field_hover, ring, Pointer};
use crate::ui::shell::Shell;

/// ISO, as the draft and the field names write it.
pub(super) const FORMAT: &str = "%Y-%m-%d";

/// The two pickers; each starts on the draft's day.
pub(super) struct Dates {
    pub start: Entity<DatePickerState>,
    pub end: Entity<DatePickerState>,
    /// The text each day field last held, start then end. It is the draft's own text until a day is picked, so a
    /// draft date that cannot be read (the picker stays empty) is still shown, kept and refused on save.
    shown: [String; 2],
}

impl Dates {
    pub fn new(start: &str, end: &str, window: &mut Window, cx: &mut Context<Shell>) -> Self {
        let make = |text: &str, window: &mut Window, cx: &mut Context<Shell>| {
            let day = parse_date(text);
            cx.new(|cx| {
                let mut s = DatePickerState::new(window, cx).date_format(FORMAT);
                if let Some(day) = day {
                    s.set_date_time(day.and_time(NaiveTime::MIN), window, cx);
                }
                s
            })
        };
        Self { start: make(start, window, cx), end: make(end, window, cx), shown: [start.to_string(), end.to_string()] }
    }

    fn slot(f: Field) -> usize {
        usize::from(f == Field::EndDate)
    }

    /// `None` for a field that is not a date.
    pub fn of(&self, f: Field) -> Option<&Entity<DatePickerState>> {
        match f {
            Field::StartDate => Some(&self.start),
            Field::EndDate => Some(&self.end),
            _ => None,
        }
    }

    /// The day shown, `2026-10-28`; the draft's own text while no day is picked.
    pub fn text(&self, f: Field, cx: &App) -> String {
        let day = self.of(f).and_then(|s| s.read(cx).date_time().format(FORMAT));
        day.map_or_else(|| self.shown[Self::slot(f)].clone(), |d| d.to_string())
    }

    pub fn focus(&self, f: Field, cx: &App) -> Option<FocusHandle> {
        self.of(f).map(|s| s.focus_handle(cx))
    }

    /// Picking another day clears that field's error, which described the day as it was; picking the same day again keeps it.
    pub fn watch(&self, window: &mut Window, cx: &mut Context<Shell>) -> Vec<Subscription> {
        [Field::StartDate, Field::EndDate]
            .into_iter()
            .filter_map(|field| {
                let state = self.of(field)?;
                Some(cx.subscribe_in(state, window, move |this, _, DatePickerEvent::Change(day): &DatePickerEvent, _, cx| {
                    let Some(f) = this.pop.as_mut() else { return };
                    let text = day.format(FORMAT).map(|d| d.to_string()).unwrap_or_default();
                    if f.dates.shown[Self::slot(field)] != text {
                        f.dates.shown[Self::slot(field)] = text;
                        f.errors.retain(|e| e.field != field);
                        cx.notify();
                    }
                }))
            })
            .collect()
    }
}

/// The picker in a `FIELD_H` box of our own. The kit's picker fixes its button at 32px and takes no height from outside, so it is
/// drawn without its frame (`appearance(false)`) and the frame is ours: the `field` fill and `border` outline, a 2px `ink` border
/// for an error, and the `ring` while the field itself has keyboard focus (not a mouse click, not while its calendar is open).
/// A cover over the whole box takes the click (it stops it, so the kit's button never sees one), focuses the picker and sends it the `Confirm` its
/// Enter key sends, which opens or closes it: no click lands in a dead band, whatever size the kit's button is.
#[derive(IntoElement)]
struct DateBox {
    id: &'static str,
    state: Entity<DatePickerState>,
    placeholder: String,
    error: bool,
    /// `border`, `field`, `t1`, `ring`, `surface` (the ring's gap), `ink`.
    colors: [Hsla; 6],
}

impl RenderOnce for DateBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let [border, field, text, ring_color, gap, ink] = self.colors;
        let handle = self.state.focus_handle(cx);
        let keyboard = window.last_input_was_keyboard() && handle.is_focused(window);
        let cover = div().id(SharedString::from(format!("{}-cover", self.id))).test_support().absolute().inset_0().pointer().on_click(move |_, window, cx| {
            // Not `occlude()`, which would also hide the box from `field_hover`; the kit's button below must not toggle it a second time.
            cx.stop_propagation();
            handle.focus(window, cx);
            handle.dispatch_action(&Confirm { secondary: false }, window, cx);
        });
        // The cover sits on this unbordered wrapper: inside the frame it would stop short of the border.
        let frame = div()
            .h(px(FIELD_H))
            .flex()
            .items_center()
            .rounded(px(FIELD_R))
            .border_1()
            .border_color(if keyboard { ring_color } else { border })
            .when(self.error, |d| d.border_2().border_color(ink))
            .bg(field)
            .text_color(text)
            .when(keyboard, |d| d.shadow(ring(gap, ring_color)))
            .child(div().w_full().child(DatePicker::new(&self.state).placeholder(self.placeholder).appearance(false)));
        div().relative().child(frame).child(cover)
    }
}

impl Shell {
    /// A labelled day field: the picker's own button-like trigger, named with the day it shows
    /// ("Start date, 2026-10-28"); an error sits under it like a text field's.
    pub(super) fn date_field(&self, t: &Theme, f: Field, (name, id): (&'static str, &'static str), form: &Form, calm: bool, cx: &App) -> AnyElement {
        let Some(state) = form.dates.of(f) else { return div().into_any_element() };
        let error = form.error(f);
        div()
            .id(SharedString::from(format!("{id}-field")))
            .test_support()
            .role(Role::Group)
            .aria_label(format!("{name}, {}", form.dates.text(f, cx)))
            .flex_1()
            .min_w_0()
            .mb(px(if error.is_some() { 4. } else { 16. }))
            .child(label(t, name))
            // An unreadable draft day shows as the placeholder, so it can be seen and replaced.
            .child(field_hover(t, format!("{id}-box"), px(FIELD_R), calm && error.is_none(), DateBox { id, state: state.clone(), placeholder: form.dates.text(f, cx), error: error.is_some(), colors: [c(t.neutral.border), c(t.neutral.field), c(t.neutral.t1), c(t.ring), c(t.neutral.surface), c(t.ink)] }))
            .when_some(error, |d, e| d.child(error_note(t, id, e)))
            .into_any_element()
    }

    /// The element id of a date field's picker, for tests.
    pub fn popover_date_id(&self, f: Field) -> Option<ElementId> {
        let state = self.pop.as_ref()?.dates.of(f)?;
        // gpui-kit builds this id inside DatePicker and exposes no way to read it, so it is spelled out here.
        Some(("date-picker", state.entity_id()).into())
    }

    /// The focus handle of a date field, for tests.
    pub fn popover_date_focus(&self, f: Field, cx: &App) -> Option<FocusHandle> {
        self.pop.as_ref()?.dates.focus(f, cx)
    }
}
