//! Drawing the popover and the "Event deleted. Undo" toast.

use std::time::Duration;

use gpui_kit::{
    Animation, AnimationExt, AnyElement, ClickEvent, Anchor, App, Context, FocusHandle, Focusable, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, anchored, deferred, div, ease_out_quint, point,
    px, prelude::FluentBuilder,
};

use super::Form;
use super::fields::{PAD_X, STACK_BELOW, notes_field, section, status_box, text_field};
use super::place::{MARGIN, Side, below_header, card_width, side};
use crate::domain::form::{Field, summary};
use crate::ui::controls::{c, control, elevation, ink_hover, ink_press, scrim};
use crate::ui::shell::Shell;
use crate::theme::resolve::Theme;

const FADE_MS: u64 = 160;
/// The well when the dates and times do not make an event.
const WELL_FALLBACK: &str = "Check the dates and times";

impl Shell {
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
        let (well_head, well_sub) = summary(&draft, self.model.today, hour24).unwrap_or_else(|| (WELL_FALLBACK.into(), String::new()));
        let (i, ev) = (&form.inputs, &form);
        let pair = |a: AnyElement, b: AnyElement| div().flex().gap(px(14.)).child(a).child(b);
        // The field with focus keeps its ring: only the others hover in `t2`.
        let calm = |h: Option<FocusHandle>, window: &Window, cx: &App| !h.is_some_and(|h| h.contains_focused(window, cx));
        let [calm_title, calm_notes] = [i.title.focus_handle(cx), i.notes.focus_handle(cx)].map(|h| calm(Some(h), window, cx));
        let [calm_sd, calm_st, calm_ed, calm_et] = [Field::StartDate, Field::StartTime, Field::EndDate, Field::EndTime].map(|f| calm(ev.dates.focus(f, cx).or_else(|| ev.times.focus(f)), window, cx));
        let day = |this: &Self, f, name, id, calm| this.date_field(t, f, (name, id), ev, calm, cx);
        let (start_day, end_day) = (day(self, Field::StartDate, "Start date", "pop-start-date", calm_sd), day(self, Field::EndDate, "End date", "pop-end-date", calm_ed));
        // Greyed while the event is all day; the draft keeps the times.
        let time = |f, name, id, calm| self.time_field(t, f, (name, id), ev, calm, window, cx);
        let start = pair(start_day, time(Field::StartTime, "Start time", "pop-start-time", calm_st));
        let end = pair(end_day, time(Field::EndTime, "End time", "pop-end-time", calm_et));
        let title = text_field(t, Field::Title, "Title", "pop-title", &i.title, ev, calm_title);
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
        let input = |f: Field, cx: &mut Context<Self>| form.inputs.of(f).map(|i| i.focus_handle(cx)).or_else(|| form.dates.focus(f, cx)).or_else(|| form.times.focus(f));
        let mut order: Vec<_> = input(Field::Title, cx).into_iter().collect();
        order.push(keyed(format!("pop-cal-{}", draft.calendar), window, cx));
        let all_day_handle = keyed("pop-all-day".into(), window, cx);
        *form.all_day.borrow_mut() = Some(all_day_handle.clone());
        order.push(all_day_handle);
        order.extend(input(Field::StartDate, cx));
        order.extend(if draft.all_day { None } else { input(Field::StartTime, cx) });
        order.extend(input(Field::EndDate, cx));
        order.extend(if draft.all_day { None } else { input(Field::EndTime, cx) });
        order.push(form.inputs.notes.focus_handle(cx));
        order.extend(existing.then(|| keyed("pop-delete".into(), window, cx)));
        order.push(keyed("pop-cancel".into(), window, cx));
        order.push(keyed("pop-save".into(), window, cx));
        *form.order.borrow_mut() = order;

        let well_box = status_box(t, &well_head, &well_sub);
        let notes = notes_field(t, ev, calm_notes);
        // The card never outgrows the room it is placed in (its body scrolls instead). With no trigger it is centered
        // in the main card below the header; until that is painted, in the window.
        let centered = if form.anchor.is_none() { self.anchors.first("main").map(below_header) } else { None };
        let room = centered.map_or_else(|| (window.viewport_size().height - px(2. * MARGIN)).max(px(0.)), |a| a.size.height);
        let width = card_width(f32::from(centered.map_or_else(|| window.viewport_size().width, |a| a.size.width)));
        let stacked = width < STACK_BELOW;
        let heading = if existing { "Edit event" } else { "New event" };
        let subtitle = if existing { "Change it, then save." } else { "Add it to one of your calendars." };
        let card = div()
            .id("popover")
            .test_support()
            .role(Role::Dialog)
            .aria_label(if existing { "Edit event form" } else { "New event form" })
            .key_context(super::KEY_CONTEXT)
            .track_focus(&form.trap)
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.card_pressed(cx)))
            .on_action(cx.listener(|this, _: &super::TrapNext, window, cx| this.trap_tab(false, window, cx)))
            .on_action(cx.listener(|this, _: &super::TrapPrev, window, cx| this.trap_tab(true, window, cx)))
            .w(px(width))
            .max_h(room)
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
                    .px(px(PAD_X))
                    .pt(px(20.))
                    .pb(px(4.))
                    .child(div().text_size(px(20.)).font_weight(FontWeight::BOLD).child(heading))
                    .child(div().id("pop-subtitle").test_support().aria_label(subtitle).mt(px(2.)).text_size(px(12.)).text_color(c(n.t2)).child(subtitle)),
            )
            .child(
                div()
                    .id("pop-body")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(section(t, stacked, "pop-sec-details", "Details", "What it is and where it lives.", div().child(title).child(cards)))
                    .child(section(t, stacked, "pop-sec-when", "When", "Drag on the grid to set this, or pick it.", div().child(all_day).child(start).child(end).child(well_box)))
                    .child(section(t, stacked, "pop-sec-notes", "Notes", "Optional.", notes)),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(PAD_X))
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
            // No trigger: centered in the main card (the window until it has been painted), clear of the header title.
            None => {
                let area = div().absolute().flex().items_center().justify_center().child(card);
                match centered {
                    Some(a) => area.left(a.left()).top(a.top()).w(a.size.width).h(a.size.height),
                    None => area.inset_0(),
                }
                .into_any_element()
            }
        };
        // The backdrop swallows pointer input: the page behind is inert while the card is open; a click on it closes.
        let backdrop = div()
            .id("pop-backdrop")
            .test_support()
            .absolute()
            .inset_0()
            .occlude()
            .when(form.anchor.is_none(), |d| d.bg(scrim(t)))
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
            .hover(ink_hover(t))
            .active(ink_press(t))
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
