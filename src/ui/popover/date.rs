//! The start and end days of the popover: picked from gpui-kit's date picker, never typed.

use chrono::NaiveTime;
use gpui_kit::component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_kit::{
    AnyElement, App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled,
    Subscription, TestSupportExt, Window, div, prelude::*, px,
};

use super::Form;
use super::fields::{FIELD_H, error_note, label};
use crate::domain::form::{Field, parse_date};
use crate::theme::resolve::Theme;
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

impl Shell {
    /// A labelled day field: the picker's own button-like trigger, named with the day it shows
    /// ("Start date, 2026-10-28"); an error sits under it like a text field's.
    pub(super) fn date_field(&self, t: &Theme, f: Field, name: &'static str, id: &'static str, form: &Form, cx: &App) -> AnyElement {
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
            .child(DatePicker::new(state).placeholder(form.dates.text(f, cx)).h(px(FIELD_H)))
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
