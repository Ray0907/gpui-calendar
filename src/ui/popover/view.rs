//! Drawing the popover and the "Event deleted. Undo" toast.

use std::time::Duration;

use gpui_kit::component::IconName;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, ClickEvent, Anchor, Context, Entity, FocusHandle, Focusable, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Toggled, Window, anchored, deferred, div, ease_out_quint, point,
    prelude::*, px,
};

use super::place::{MARGIN, Side, WIDTH, side};
use super::Form;
use crate::domain::form::{Field, summary};
use crate::theme::gpui_adapter::shade;
use crate::ui::controls::{c, control, elevation, icon, shape, shape_of};
use crate::ui::shell::Shell;
use crate::theme::resolve::Theme;

const FADE_MS: u64 = 160;
pub(super) const FIELD_H: f32 = 48.;

pub(super) fn label(t: &Theme, text: &'static str) -> impl IntoElement {
    div().mb(px(6.)).text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(c(t.neutral.t1)).child(text)
}

/// The message under a field, read as an alert.
pub(super) fn error_note(t: &Theme, id: &str, e: &'static str) -> impl IntoElement {
    div().id(SharedString::from(format!("{id}-error"))).test_support().role(Role::Alert).aria_label(e).mt(px(4.)).mb(px(12.)).text_size(px(12.)).text_color(c(t.neutral.t1)).child(e)
}

impl Shell {
    /// A labelled 48px text field; when `error` is set the field gets the ink border and the message
    /// sits under it (read as its description).
    fn text_field(&self, t: &Theme, field: Field, name: &'static str, id: &'static str, state: &Entity<InputState>, form: &Form) -> AnyElement {
        let error = form.error(field);
        let input = Input::new(state)
            .id(id)
            .aria_label(name)
            .h(px(FIELD_H))
            .rounded(px(10.))
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

    fn calendar_cards(&mut self, t: &Theme, selected: u8, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let calendars = self.model.store.calendars.clone();
        // Taken while painting: key handlers cannot reach the keyed state.
        let handles: Vec<(u8, FocusHandle)> =
            calendars.iter().map(|k| (k.id, window.use_keyed_state(format!("pop-cal-{}", k.id), cx, |_, cx| cx.focus_handle()).read(cx).clone())).collect();
        let cards: Vec<_> = calendars
            .iter()
            .map(|k| {
                let id = k.id;
                let handles = handles.clone();
                let on = id == selected;
                let n = self.model.store.events.iter().filter(|e| e.calendar == id).count();
                let dot = t.cal(id).map_or(c(t.ink), |k| c(k.dot));
                // Roving: the chosen card is the group's one Tab stop, the arrows move between cards.
                let handle = handles.iter().find(|(i, _)| *i == id).map(|(_, h)| h.clone()).expect("every card has a handle");
                control(format!("pop-cal-{id}"), Role::RadioButton, k.name.clone(), c(t.neutral.surface), c(t.ring), window, cx)
                    .track_focus(&handle.tab_stop(on))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .aria_description(format!("{n} events"))
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .p(px(if on { 11. } else { 12. }))
                    .rounded(px(12.))
                    .border(px(if on { 2. } else { 1. }))
                    .border_color(if on { c(t.ink) } else { c(t.neutral.border) })
                    .when(on, |s| s.bg(c(t.neutral.subtle)))
                    .child(div().flex().items_center().gap(px(7.)).text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(shape(shape_of(id), dot)).child(k.name.clone()))
                    .child(div().mt(px(2.)).text_size(px(12.)).text_color(c(t.neutral.t3)).child(format!("{n} event{}", if n == 1 { "" } else { "s" })))
                    .when(on, |s| {
                        s.child(div().absolute().top(px(8.)).right(px(8.)).size(px(18.)).rounded_full().bg(c(t.ink)).flex().items_center().justify_center().child(icon(IconName::Check, c(t.on_ink), 12.)))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_form_calendar(id, cx)))
                    .on_key_down(cx.listener(move |this, e: &KeyDownEvent, window, cx| {
                        let forward = match e.keystroke.key.as_str() {
                            "left" | "up" => false,
                            "right" | "down" => true,
                            _ => return,
                        };
                        // Cards are found by position: ids need not be contiguous.
                        let all = &this.model.store.calendars;
                        let Some(at) = all.iter().position(|k| k.id == id) else { return };
                        let to = all[(at + if forward { 1 } else { all.len() - 1 }) % all.len()].id;
                        this.set_form_calendar(to, cx);
                        if let Some((_, h)) = handles.iter().find(|(i, _)| *i == to) {
                            h.focus(window, cx);
                        }
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

    fn all_day_toggle(&mut self, t: &Theme, on: bool, dot: gpui_kit::Hsla, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
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

    fn footer_button(&mut self, t: &Theme, id: &'static str, name: &'static str, primary: bool, window: &mut Window, cx: &mut Context<Self>) -> impl StatefulInteractiveElement + Styled + ParentElement + IntoElement + use<> {
        control(id, Role::Button, name, c(t.neutral.subtle), c(t.ring), window, cx)
            .h(px(40.))
            .px(px(18.))
            .flex()
            .items_center()
            .rounded(px(10.))
            .font_weight(FontWeight::SEMIBOLD)
            .when(primary, |s| s.bg(c(t.ink)).text_color(c(t.on_ink)))
            .when(!primary, |s| s.bg(c(t.neutral.surface)).border_1().border_color(c(t.neutral.border)))
            .child(name)
    }

    /// The card, its backdrop, and the placement. `None` when nothing is open.
    pub(in crate::ui) fn popover_layer(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let form = self.pop.take()?;
        let result = self.popover_card(t, &form, window, cx);
        self.pop = Some(form);
        Some(result)
    }

    fn popover_card(&mut self, t: &Theme, form: &Form, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let hour24 = self.model.store.settings.time_format.hour24();
        let draft = form.read(cx);
        let existing = draft.id.is_some();
        let dot = t.cal(draft.calendar).map_or(c(t.ink), |k| c(k.dot));
        let n = &t.neutral;
        let well = summary(&draft, self.model.today, hour24);
        let (i, ev) = (&form.inputs, &form);
        let pair = |a: AnyElement, b: AnyElement| div().flex().gap(px(14.)).child(a).child(b);
        let day = |this: &Self, f, name, id| this.date_field(t, f, name, id, ev, cx);
        let time = |this: &Self, f, name, id, s: &Entity<InputState>| this.text_field(t, f, name, id, s, ev);
        let (start_day, end_day) = (day(self, Field::StartDate, "Start date", "pop-start-date"), day(self, Field::EndDate, "End date", "pop-end-date"));
        let (start, end) = if draft.all_day {
            (start_day, end_day)
        } else {
            (
                pair(start_day, time(self, Field::StartTime, "Start time", "pop-start-time", &i.start_time)).into_any_element(),
                pair(end_day, time(self, Field::EndTime, "End time", "pop-end-time", &i.end_time)).into_any_element(),
            )
        };
        let title = self.text_field(t, Field::Title, "Title", "pop-title", &i.title, ev);
        let cards = self.calendar_cards(t, draft.calendar, window, cx);
        let all_day = self.all_day_toggle(t, draft.all_day, dot, window, cx);
        let delete = existing.then(|| {
            control("pop-delete", Role::Button, "Delete event", c(n.subtle), c(t.ring), window, cx)
                .h(px(40.))
                .px(px(4.))
                .flex()
                .items_center()
                .text_color(c(n.t2))
                .underline()
                .child("Delete event")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.delete_from_form(window, cx)))
        });
        let cancel = self.footer_button(t, "pop-cancel", "Cancel", false, window, cx).on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_popover(window, cx)));
        let saver = self.footer_button(t, "pop-save", "Save", true, window, cx).on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.submit_form(window, cx)));

        // The tab stops in reading order, for the focus trap.
        let keyed = |id: String, window: &mut Window, cx: &mut Context<Self>| window.use_keyed_state(SharedString::from(id), cx, |_, cx| cx.focus_handle()).read(cx).clone();
        let input = |f: Field, cx: &mut Context<Self>| form.inputs.of(f).map(|i| i.focus_handle(cx)).or_else(|| form.dates.focus(f, cx));
        let mut order: Vec<_> = input(Field::Title, cx).into_iter().collect();
        order.push(keyed(format!("pop-cal-{}", draft.calendar), window, cx));
        order.extend(input(Field::StartDate, cx));
        order.extend(if draft.all_day { None } else { input(Field::StartTime, cx) });
        order.extend(input(Field::EndDate, cx));
        order.extend(if draft.all_day { None } else { input(Field::EndTime, cx) });
        order.push(keyed("pop-all-day".into(), window, cx));
        order.extend(existing.then(|| keyed("pop-delete".into(), window, cx)));
        order.push(keyed("pop-cancel".into(), window, cx));
        order.push(keyed("pop-save".into(), window, cx));
        *form.order.borrow_mut() = order;

        let heading = if existing { "Edit event" } else { "New event" };
        let card = div()
            .id("popover")
            .test_support()
            .role(Role::Dialog)
            .aria_label(if existing { "Edit event form" } else { "New event form" })
            .key_context(super::KEY_CONTEXT)
            .track_focus(&form.trap)
            .on_action(cx.listener(|this, _: &super::TrapNext, window, cx| this.trap_tab(false, window, cx)))
            .on_action(cx.listener(|this, _: &super::TrapPrev, window, cx| this.trap_tab(true, window, cx)))
            .w(px(WIDTH))
            .occlude() // a click on the card is not a click on the backdrop behind it
            .flex()
            .flex_col()
            .rounded(px(14.))
            .overflow_hidden()
            .bg(c(n.surface))
            .text_color(c(n.t1))
            .text_size(px(13.))
            .shadow(elevation(t))
            .child(div().id("pop-bar").test_support().h(px(6.)).flex_none().bg(dot))
            .child(
                div()
                    .px(px(24.))
                    .pt(px(16.))
                    .pb(px(12.))
                    .child(div().text_size(px(20.)).font_weight(FontWeight::BOLD).child(heading))
                    .child(div().mt(px(2.)).text_size(px(12.)).text_color(c(n.t2)).child(if existing { "Change it, then save." } else { "Add it to one of your calendars." })),
            )
            .child(div().px(px(24.)).child(title).child(cards).child(start).child(end).child(all_day).child(
                div().mt(px(4.)).mb(px(16.)).rounded(px(12.)).bg(c(n.page)).px(px(14.)).py(px(10.)).child(
                    div()
                        .id("pop-well")
                        .test_support()
                        .role(Role::Status)
                        .aria_label(well.as_ref().map_or("Check the dates and times".to_string(), |w| format!("{}. {}", w.0, w.1)))
                        .child(div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).child(well.as_ref().map_or("Check the dates and times".to_string(), |w| w.0.clone())))
                        .child(div().text_size(px(12.)).text_color(c(n.t2)).child(well.map_or(String::new(), |w| w.1))),
                ),
            ))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(24.))
                    .py(px(16.))
                    .border_t_1()
                    .border_color(c(n.line))
                    .bg(c(n.subtle))
                    .children(delete)
                    .child(div().flex_1())
                    .child(cancel)
                    .child(saver),
            );
        let card: AnyElement = if form.by_key {
            card.into_any_element()
        } else {
            card.with_animation(("popover-in", form.seq), Animation::new(Duration::from_millis(FADE_MS)).with_easing(ease_out_quint()), |el, d| el.opacity(d)).into_any_element()
        };
        let placed = match form.anchor {
            Some(a) => {
                let (pos, corner) = match side((f32::from(a.left()), f32::from(a.right())), f32::from(window.viewport_size().width)) {
                    Side::Right(x) => (x, Anchor::TopLeft),
                    Side::Left(x) => (x, Anchor::TopRight),
                };
                anchored().position(point(px(pos), a.top())).anchor(corner).snap_to_window_with_margin(px(MARGIN)).child(card).into_any_element()
            }
            None => div().absolute().inset_0().flex().items_center().justify_center().child(card).into_any_element(),
        };
        // The backdrop swallows pointer input: the page behind is inert while the card is open; a click on it closes.
        let backdrop = div()
            .id("pop-backdrop")
            .test_support()
            .absolute()
            .inset_0()
            .occlude()
            .bg(shade(if form.anchor.is_none() { 0.18 } else { 0. }))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_popover(window, cx)));
        deferred(div().absolute().inset_0().child(backdrop).child(placed)).with_priority(2).into_any_element()
    }

    /// "Event deleted." with an Undo button, centered at the bottom.
    pub(in crate::ui) fn toast_layer(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = self.toast.clone()?;
        let undo = control("toast-undo", Role::Button, "Undo", c(t.ink), c(t.on_ink), window, cx)
            .h(px(28.))
            .px(px(10.))
            .flex()
            .items_center()
            .rounded(px(8.))
            .font_weight(FontWeight::SEMIBOLD)
            .underline()
            .child("Undo")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.undo_last(cx)));
        let bar = div()
            .id("toast")
            .test_support()
            .role(Role::Status)
            .aria_label(format!("{text} Undo available"))
            .flex()
            .items_center()
            .gap(px(8.))
            .pl(px(16.))
            .pr(px(6.))
            .py(px(6.))
            .rounded(px(10.))
            .bg(c(t.ink))
            .text_color(c(t.on_ink))
            .text_size(px(13.))
            .shadow(elevation(t))
            .child(text)
            .child(undo);
        Some(deferred(div().absolute().bottom(px(24.)).left_0().right_0().flex().justify_center().child(bar)).with_priority(3).into_any_element())
    }
}
