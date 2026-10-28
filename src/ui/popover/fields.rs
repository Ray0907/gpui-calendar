//! The popover's fields: text inputs, calendar cards, the all-day switch and the footer buttons.

use gpui_kit::component::IconName;
use gpui_kit::component::input::{Input, InputState, Textarea};
use gpui_kit::{
    AnyElement, ClickEvent, Context, Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled,
    TestSupportExt, Toggled, Window, div, prelude::*, px,
};

use super::Form;
use crate::domain::form::Field;
use crate::ui::controls::{c, control, focus_ring, icon, shape, shape_of};
use crate::ui::shell::Shell;
use crate::theme::resolve::Theme;

pub(super) const FIELD_H: f32 = 48.;
pub(super) const NOTES_H: f32 = 72.;
/// Corner radius of inputs and buttons; the horizontal padding of the header, sections and footer.
pub(super) const FIELD_R: f32 = 10.;
pub(super) const PAD_X: f32 = 28.;
/// The hint column beside a section's fields; below `STACK_BELOW` card width the hint sits above them instead.
const HINT_W: f32 = 190.;
pub(super) const STACK_BELOW: f32 = 560.;

pub(super) fn label(t: &Theme, text: &'static str) -> impl IntoElement {
    div().mb(px(6.)).text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(c(t.neutral.t1)).child(text)
}

/// The message under a field, read as an alert.
pub(super) fn error_note(t: &Theme, id: &str, e: &'static str) -> impl IntoElement {
    div().id(SharedString::from(format!("{id}-error"))).test_support().role(Role::Alert).aria_label(e).mt(px(4.)).mb(px(12.)).text_size(px(12.)).text_color(c(t.neutral.t1)).child(e)
}

/// A labelled 48px text field; when `error` is set the field gets the ink border and the message
/// sits under it (read as its description).
pub(super) fn text_field(t: &Theme, field: Field, name: &'static str, id: &'static str, state: &Entity<InputState>, form: &Form) -> AnyElement {
    let error = form.error(field);
    let input = Input::new(state)
        .id(id)
        .aria_label(name)
        .h(px(FIELD_H))
        .rounded(px(FIELD_R))
        .when(error.is_some(), |i| i.border_2().border_color(c(t.ink)));
    div()
        .id(SharedString::from(format!("{id}-field")))
        .test_support()
        .role(Role::Group)
        .aria_label(name)
        .when_some(error, |d, e| d.aria_description(e))
        .flex_1()
        .min_w_0()
        .mb(px(if error.is_some() { 4. } else { 16. }))
        .child(label(t, name))
        .child(input)
        .when_some(error, |d, e| d.child(error_note(t, id, e)))
        .into_any_element()
}

impl Shell {
    pub(super) fn calendar_cards(&mut self, t: &Theme, selected: u8, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let calendars: Vec<(u8, SharedString)> = self.model.store.calendars.iter().map(|k| (k.id, SharedString::from(k.name.clone()))).collect();
        // Taken while painting: key handlers cannot reach the keyed state.
        let handles: Vec<FocusHandle> = calendars.iter().map(|(id, _)| window.use_keyed_state(format!("pop-cal-{id}"), cx, |_, cx| cx.focus_handle()).read(cx).clone()).collect();
        let len = calendars.len();
        let events = &self.model.store.events;
        let cards: Vec<_> = calendars
            .iter()
            .enumerate()
            .map(|(at, (id, name))| {
                let id = *id;
                let on = id == selected;
                let n = events.iter().filter(|e| e.calendar == id).count();
                let dot = t.cal(id).map_or(c(t.ink), |k| c(k.dot));
                // Cards are found by position: ids need not be contiguous. Each handler holds only its two neighbours.
                let (prev, next) = ((calendars[(at + len - 1) % len].0, handles[(at + len - 1) % len].clone()), (calendars[(at + 1) % len].0, handles[(at + 1) % len].clone()));
                // Roving: the chosen card is the group's one Tab stop, the arrows move between cards.
                control(format!("pop-cal-{id}"), Role::RadioButton, name.clone(), c(t.neutral.surface), c(t.ring), window, cx)
                    .track_focus(&handles[at].clone().tab_stop(on))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .aria_description(format!("{n} events"))
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .p(px(if on { 11. } else { 12. }))
                    .rounded(px(12.))
                    .border(px(if on { 2. } else { 1. }))
                    .border_color(if on { c(t.ink) } else { c(t.neutral.border) })
                    .when(on, |s| s.bg(c(t.neutral.subtle)).focus_visible(focus_ring(c(t.neutral.surface), c(t.ring), c(t.neutral.subtle))))
                    .child(div().flex().items_center().gap(px(7.)).text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(shape(shape_of(id), dot)).child(name.clone()))
                    .child(div().mt(px(2.)).text_size(px(12.)).text_color(c(t.neutral.t3)).child(format!("{n} event{}", if n == 1 { "" } else { "s" })))
                    .when(on, |s| {
                        s.child(div().absolute().top(px(8.)).right(px(8.)).size(px(18.)).rounded_full().bg(c(t.ink)).flex().items_center().justify_center().child(icon(IconName::Check, c(t.on_ink), 12.)))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_form_calendar(id, cx)))
                    .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                        let (to, handle) = match e.keystroke.key.as_str() {
                            "left" | "up" => &prev,
                            "right" | "down" => &next,
                            _ => return,
                        };
                        this.set_form_calendar(*to, cx);
                        handle.focus(window, cx);
                        cx.stop_propagation();
                    }))
                    .into_any_element()
            })
            .collect();
        div()
            .id("pop-calendar")
            .test_support()
            .role(Role::RadioGroup)
            .aria_label("Calendar")
            .mb(px(16.))
            .child(label(t, "Calendar"))
            .child(div().flex().gap(px(10.)).children(cards))
            .into_any_element()
    }

    pub(super) fn all_day_toggle(&mut self, t: &Theme, on: bool, dot: gpui_kit::Hsla, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        control("pop-all-day", Role::CheckBox, "All day", c(t.neutral.surface), c(t.ring), window, cx)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .h(px(36.))
            .mb(px(8.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .size(px(16.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(if on { dot } else { c(t.neutral.border) })
                    .when(on, |s| s.bg(dot))
                    .children(on.then(|| icon(IconName::Check, c(t.on_ink), 12.))),
            )
            .child(div().text_size(px(13.)).child("All day"))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.toggle_form_all_day(window, cx)))
            .into_any_element()
    }

    pub(super) fn footer_button(&mut self, t: &Theme, id: &'static str, name: &'static str, primary: bool, window: &mut Window, cx: &mut Context<Self>) -> impl StatefulInteractiveElement + Styled + ParentElement + IntoElement + use<> {
        control(id, Role::Button, name, c(t.neutral.subtle), c(t.ring), window, cx)
            .h(px(40.))
            .px(px(18.))
            .flex()
            .items_center()
            .rounded(px(FIELD_R))
            .font_weight(FontWeight::SEMIBOLD)
            .when(primary, |s| s.bg(c(t.ink)).text_color(c(t.on_ink)).focus_visible(focus_ring(c(t.neutral.subtle), c(t.ring), c(t.ink))))
            .when(!primary, |s| s.bg(c(t.neutral.surface)).border_1().border_color(c(t.neutral.border)).focus_visible(focus_ring(c(t.neutral.subtle), c(t.ring), c(t.neutral.surface))))
            .child(name)
    }
}

/// One form section as in the mock: a heading and its hint on the left, the fields on the right.
pub(super) fn section(t: &Theme, stacked: bool, id: &'static str, title: &'static str, hint: &'static str, body: impl IntoElement) -> impl IntoElement {
    let n = &t.neutral;
    div()
        .id(id)
        .test_support()
        .flex()
        .when(stacked, |s| s.flex_col().gap(px(12.)))
        .when(!stacked, |s| s.gap(px(28.)))
        .px(px(PAD_X))
        .py(px(16.))
        .border_t_1()
        .border_color(c(n.line))
        .child(
            div()
                .when(!stacked, |h| h.w(px(HINT_W)).flex_none())
                .child(div().mb(px(4.)).text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(title))
                .child(div().text_size(px(12.)).line_height(px(18.)).text_color(c(n.t2)).child(hint)),
        )
        .child(div().flex_1().min_w_0().child(body))
}

/// The multi-line notes box. The group names it "Notes"; the textarea itself carries no label, so it is said once.
pub(super) fn notes_field(t: &Theme, form: &Form) -> AnyElement {
    let area = Textarea::new(&form.inputs.notes).accessibility_id("pop-notes").h(px(NOTES_H)).rounded(px(FIELD_R)).bg(c(t.neutral.surface));
    div().id("pop-notes-field").test_support().role(Role::Group).aria_label("Notes").child(area).into_any_element()
}

/// The well under the dates: the readable time over its duration, read as one status.
pub(super) fn status_box(t: &Theme, head: &str, sub: &str) -> impl IntoElement {
    let n = &t.neutral;
    let spoken = if sub.is_empty() { head.to_string() } else { format!("{head}. {sub}") };
    div().mb(px(6.)).rounded(px(12.)).bg(c(n.page)).px(px(14.)).py(px(12.)).child(
        div()
            .id("pop-well")
            .test_support()
            .role(Role::Status)
            .aria_label(spoken)
            .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(head.to_string()))
            .child(div().text_size(px(12.)).text_color(c(n.t2)).child(sub.to_string())),
    )
}
