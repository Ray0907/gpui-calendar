//! The start and end times of the popover: picked in segments with the arrow keys from gpui-kit's time field, never typed as text.

use chrono::NaiveTime;
use gpui_kit::component::{ActiveTheme, Disableable};
use gpui_kit::component::time_field::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role, SharedString, Styled, Subscription,
    TestSupportExt, Window, div, prelude::*, px,
};

#[cfg(any(test, feature = "test-hooks"))]
use gpui_kit::ElementId;

use super::Form;
use super::fields::{FIELD_H, FIELD_R, error_note, label};
use crate::domain::dates::format_time;
use crate::domain::form::{Field, parse_time};
use crate::theme::resolve::Theme;
use crate::ui::controls::{c, field_hover};
use crate::ui::a11y::Disabled;
use crate::ui::shell::Shell;

/// The two fields; each starts on the draft's time, in the app's 12 or 24 hour clock.
pub(super) struct Times {
    start: Entity<TimeFieldState>,
    end: Entity<TimeFieldState>,
    hour24: bool,
    /// The draft's own text for a time that cannot be read (the field sits at midnight), start then end. It is shown, kept and
    /// refused on save until a time is picked.
    unread: [Option<String>; 2],
}

impl Times {
    pub fn new(start: &str, end: &str, hour24: bool, window: &mut Window, cx: &mut Context<Shell>) -> Self {
        let make = |text: &str, window: &mut Window, cx: &mut Context<Shell>| {
            let time = parse_time(text, hour24);
            let cycle = if hour24 { HourCycle::H23 } else { HourCycle::H12 };
            let state = cx.new(|cx| {
                let mut s = TimeFieldState::new(window, cx).hour_cycle(cycle);
                s.set_time(time.unwrap_or(NaiveTime::MIN), window, cx);
                s
            });
            (state, time.is_none().then(|| text.to_string()))
        };
        let ((start, a), (end, b)) = (make(start, window, cx), make(end, window, cx));
        Self { start, end, hour24, unread: [a, b] }
    }

    fn slot(f: Field) -> usize {
        usize::from(f == Field::EndTime)
    }

    /// `None` for a field that is not a time.
    pub fn of(&self, f: Field) -> Option<&Entity<TimeFieldState>> {
        match f {
            Field::StartTime => Some(&self.start),
            Field::EndTime => Some(&self.end),
            _ => None,
        }
    }

    /// True while the draft's own text for `f` stands because it could not be read.
    pub fn is_unread(&self, f: Field) -> bool {
        self.unread[Self::slot(f)].is_some()
    }

    /// The time shown, `14:30` or `2:30 PM`; the draft's own text while no time is picked.
    pub fn text(&self, f: Field, cx: &App) -> String {
        let time = self.of(f).map(|s| s.read(cx).time());
        match (&self.unread[Self::slot(f)], time) {
            (Some(text), _) => text.clone(),
            (None, Some(t)) => format_time(t, self.hour24),
            (None, None) => String::new(),
        }
    }

    /// Show `text` as picking it would: a time that reads is set, one that does not is kept as the draft's text.
    pub fn set(&mut self, f: Field, text: &str, window: &mut Window, cx: &mut Context<Shell>) {
        let Some(state) = self.of(f).cloned() else { return };
        let time = parse_time(text, self.hour24);
        if let Some(t) = time {
            state.update(cx, |s, cx| s.set_time(t, window, cx));
        }
        self.unread[Self::slot(f)] = time.is_none().then(|| text.to_string());
    }

    pub fn focus(&self, f: Field, cx: &App) -> Option<FocusHandle> {
        self.of(f).map(|s| s.focus_handle(cx))
    }

    /// Picking a time clears that field's error, which described the time as it was.
    pub fn watch(&self, window: &mut Window, cx: &mut Context<Shell>) -> Vec<Subscription> {
        [Field::StartTime, Field::EndTime]
            .into_iter()
            .filter_map(|field| {
                let state = self.of(field)?;
                Some(cx.subscribe_in(state, window, move |this, _, _: &TimeFieldEvent, _, cx| {
                    let Some(f) = this.pop.as_mut() else { return };
                    f.times.unread[Self::slot(field)] = None;
                    f.errors.retain(|e| e.field != field);
                    cx.notify();
                }))
            })
            .collect()
    }
}

impl Shell {
    /// A labelled time field, 48px like the others; disabled (greyed, not in the tab order) while the event is all day. An error
    /// sits under it like a text field's.
    pub(super) fn time_field(&self, t: &Theme, f: Field, (name, id): (&'static str, &'static str), form: &Form, calm: bool, cx: &Context<Self>) -> AnyElement {
        let Some(state) = form.times.of(f) else { return div().into_any_element() };
        let (error, off) = (form.error(f), form.draft.all_day);
        let field = TimeField::new(state)
            .w_full()
            .h(px(FIELD_H))
            .rounded(px(FIELD_R))
            .disabled(off)
            // A time the draft held but nobody can read sits at 00:00: invalid until one is picked.
            .invalid(form.times.is_unread(f))
            // `disabled` keeps the fill (state table): the library would swap it for a tinted one.
            .when(off, |i| i.bg(cx.theme().input_background()))
            .when(error.is_some(), |i| i.border_2().border_color(c(t.ink)));
        Disabled::new(div().id(SharedString::from(format!("{id}-field"))), off)
            .test_support()
            .role(Role::Group)
            .aria_label(name)
            .when_some(error, |d, e| d.aria_description(e))
            .flex_1()
            .min_w_0()
            .mb(px(if error.is_some() { 4. } else { 16. }))
            // Enter saves from a time field as it does from a text field; the field itself leaves the key alone.
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                if e.keystroke.key == "enter" {
                    this.submit_form(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label(t, name))
            .child(field_hover(t, format!("{id}-box"), px(FIELD_R), calm && !off && error.is_none(), field))
            .when_some(error, |d, e| d.child(error_note(t, id, e)))
            .into_any_element()
    }

    /// The element id of a time field, for tests.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn popover_time_id(&self, f: Field) -> Option<ElementId> {
        let state = self.pop.as_ref()?.times.of(f)?;
        // gpui-component builds this id inside TimeField and exposes no way to read it, so it is spelled out here.
        Some(("time-field", state.entity_id()).into())
    }

    /// The focus handle of a time field, for tests.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn popover_time_focus(&self, f: Field, cx: &App) -> Option<FocusHandle> {
        self.pop.as_ref()?.times.focus(f, cx)
    }
}
