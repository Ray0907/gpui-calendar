//! One week row: seven day cells, then the event bars and chips laid over them by lane.

use std::ops::Range;

use chrono::{Datelike, NaiveDate, Weekday};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Role,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px, relative, prelude::*,
};

use super::{ITEM_H, ITEMS_TOP, WEEK_H};
use crate::domain::dates::format_time;
use crate::domain::event::Event;
use crate::domain::month::{Placed, item_name, layout_week};
use crate::theme::contrast::Rgb;
use crate::theme::resolve::Theme;
use crate::ui::controls::{c, ghost_hover, hover_wash, ring, shape, shape_of};
use crate::ui::popover::anchor_mark;
use crate::ui::shell::Shell;

fn is_weekend(d: NaiveDate) -> bool {
    matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

/// The fill behind whatever sits in the day cell of `d`: `subtle` on a weekend, else `surface`.
fn cell_backdrop(t: &Theme, d: NaiveDate) -> Rgb {
    if is_weekend(d) { t.neutral.subtle } else { t.neutral.surface }
}

impl Shell {
    /// The virtual list's callback: the rows for the weeks it asks for.
    pub(super) fn month_weeks(&mut self, range: Range<usize>, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = self.model.theme();
        let frame = self.month.frame.clone();
        let hour24 = self.model.store.settings.time_format.hour24();
        let shown = self.model.anchor.month();
        let (cx, grid_focused) = (&*cx, self.month.focus.is_focused(window));
        // Tabbing in lands on the anchor day until an arrow picks an event.
        let cursor = (grid_focused && self.month.roving.is_none()).then_some(self.model.anchor);
        let roving = self.month.roving.filter(|_| grid_focused);
        range
            .map(|ix| {
                let start = self.month.weeks.start_of(ix);
                let here: Vec<&Event> = frame.weeks.get(&ix).map_or(&[][..], Vec::as_slice).iter().map(|&i| &frame.events[i]).collect();
                let lay = layout_week(start, &here, self.month.cap);
                let cells: Vec<_> = (0..7)
                    .map(|col| {
                        let date = start + chrono::Duration::days(col);
                        self.day_cell(&t, date, shown, cursor == Some(date), cx)
                    })
                    .collect();
                let items: Vec<_> = lay.items.iter().map(|p| self.event_item(&t, p, here[p.ix], (ix, hour24, grid_focused), cx)).collect();
                // The roving event hidden in a count: the count carries the focus ring, on the day it starts.
                let ringed = lay.hidden.iter().find(|p| Some(p.id) == roving).map(|p| p.col);
                let more: Vec<_> = (0..7).filter(|&col| lay.more[col] > 0).map(|col| {
                    let top = ITEMS_TOP + self.month.cap as f32 * ITEM_H;
                    let text = format!("+{} more", lay.more[col]);
                    let on = ringed == Some(col);
                    let backdrop = cell_backdrop(&t, start + chrono::Duration::days(col as i64));
                    self.lane_box(top, col, 1).id(("month-more", ix * 7 + col)).test_support().role(Role::Button).aria_label(text.clone()).aria_selected(on).px(px(10.)).text_size(px(12.)).text_color(c(t.neutral.t2)).rounded(px(6.)).cursor_pointer().hover(ghost_hover(&t, backdrop)).active(|s| s.opacity(0.85)).when(on, |s| s.bg(c(t.neutral.surface)).shadow(ring(c(t.neutral.surface), c(t.ring)))).child(text).into_any_element()
                }).collect();
                div()
                    .id(("month-week", ix))
                    .relative()
                    .w_full()
                    .h(px(WEEK_H))
                    .flex()
                    .children(cells)
                    .children(items)
                    .children(more)
                    .into_any_element()
            })
            .collect()
    }

    /// An absolutely placed box over columns `col..col + span` at `top`.
    fn lane_box(&self, top: f32, col: usize, span: usize) -> gpui_kit::Div {
        div().absolute().top(px(top)).left(relative(col as f32 / 7.)).w(relative(span as f32 / 7.)).h(px(ITEM_H))
    }

    fn day_cell(&self, t: &Theme, date: NaiveDate, shown: u32, cursor: bool, cx: &Context<Self>) -> AnyElement {
        let n = &t.neutral;
        let weekend = is_weekend(date);
        let num = date.format("%-d").to_string();
        // The ring's shadow shows through a box with no fill, so the cursor's number gets the gap color (today's accent wins below).
        let number = div().min_w(px(22.)).h(px(22.)).flex().items_center().justify_center().rounded_full().when(cursor, |s| s.bg(c(n.surface))).child(num);
        let label = if date == self.model.today {
            number.bg(c(t.accent)).text_color(c(t.accent_on)).font_weight(FontWeight::SEMIBOLD)
        } else if date.month() != shown {
            number.font_weight(FontWeight::MEDIUM).text_color(c(n.t3))
        } else {
            number.font_weight(FontWeight::SEMIBOLD)
        };
        let label = label.when(cursor, |s| s.shadow(ring(c(n.surface), c(t.ring))));
        div()
            .id(SharedString::from(format!("month-cell-{date}")))
            .test_support()
            .role(Role::GridCell)
            .aria_label(date.format("%A, %B %-d, %Y").to_string())
            .aria_selected(cursor)
            .flex_1()
            .min_w_0()
            .h_full()
            .border_t_1()
            .border_color(c(n.line))
            .pt(px(6.))
            .px(px(4.))
            .when(weekend, |s| s.bg(c(n.subtle)))
            // A container, not a leaf: it washes on hover but takes no press dim, so a chip pressed inside it dims alone.
            .hover(ghost_hover(t, cell_backdrop(t, date)))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, window, cx| this.press_cell(date, window, cx)))
            .on_mouse_move(cx.listener(move |this, _: &MouseMoveEvent, _, cx| this.hover_cell(date, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(move |this, e: &MouseUpEvent, window, cx| this.release_cell(date, e.position, window, cx)))
            .child(div().h(px(22.)).mb(px(4.)).px(px(6.)).flex().items_center().justify_end().text_size(px(12.)).child(label))
            .into_any_element()
    }

    /// A chip (single-day timed event) or a tinted bar (all-day or multi-day).
    fn event_item(&self, t: &Theme, p: &Placed, e: &Event, (week, hour24, grid_focused): (usize, bool, bool), cx: &Context<Self>) -> AnyElement {
        let Some(cal) = t.cal(e.calendar) else { return div().into_any_element() };
        let (id, time) = (e.id, format_time(e.start.time(), hour24));
        let name = item_name(e, self.model.calendar_name(e.calendar), hour24, p);
        let focused = grid_focused && self.month.roving == Some(id);
        let backdrop = cell_backdrop(t, self.month.weeks.start_of(week) + chrono::Duration::days(p.col as i64));
        let inner = div()
            .id(SharedString::from(format!("month-ev-{id}-{week}")))
            .test_support()
            .role(Role::Button)
            .aria_label(name)
            .size_full()
            .flex()
            .items_center()
            .gap(px(6.))
            .my(px(1.))
            .h(px(ITEM_H - 2.))
            .rounded(px(6.))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            // The ring shadow would show through a fill at 85% opacity, so a pressed chip drops it.
            .active(|s| s.opacity(0.85).shadow(vec![]))
            .when(focused, |s| s.bg(c(t.neutral.surface)).shadow(ring(c(t.neutral.surface), c(t.ring))))
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _: &MouseDownEvent, _, _| this.press_event(id)))
            .child(anchor_mark(&self.anchors, format!("ev-{id}")));
        let inner = if p.pill {
            inner
                .px(px(8.))
                .bg(c(cal.tint))
                .hover(hover_wash(t.neutral.t1, cal.tint))
                .text_color(c(cal.text))
                .when(p.from_prev, |s| s.rounded_tl(px(0.)).rounded_bl(px(0.)))
                .when(p.to_next, |s| s.rounded_tr(px(0.)).rounded_br(px(0.)))
                .child(shape(shape_of(e.calendar), c(cal.dot)))
                .child(div().flex_1().min_w_0().truncate().child(e.title.clone()))
        } else {
            inner
                .px(px(6.))
                .hover(ghost_hover(t, backdrop))
                .child(shape(shape_of(e.calendar), c(cal.dot)))
                .child(div().flex_1().min_w_0().truncate().text_color(c(t.neutral.t1)).child(e.title.clone()))
                .child(div().flex_none().text_color(c(t.neutral.t3)).child(time))
        };
        self.lane_box(ITEMS_TOP + p.lane as f32 * ITEM_H, p.col, p.span).px(px(4.)).child(inner).into_any_element()
    }
}
