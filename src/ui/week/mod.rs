//! Week and day views: header, all-day row and the 24-hour grid. `geom` lays everything out in pixels and
//! minutes without GPUI; this file builds the elements, `input` handles the pointer and keys, `blocks`
//! draws events.

mod blocks;
mod geom;
#[cfg(test)]
mod geom_tests;
mod input;

use std::collections::HashMap;
use std::rc::Rc;

use chrono::{Datelike, NaiveDate, Timelike, Weekday};
use gpui_kit::{
    AnyElement, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Role, ScrollHandle, SharedString,
    Pixels, Point, StatefulInteractiveElement, Styled, TestSupportExt, Window, canvas, div, point, px, prelude::*,
};

use self::geom::{Area, Bar, DayLayout, GRID_H, GUTTER_W, HOUR_H, Reach, all_day_bars, all_day_height, layout_day, minute_to_y, scroll_target};
use super::controls::c;
use super::shell::Shell;
use crate::domain::dates::format_hour;
use crate::domain::drag::Drag;
use crate::domain::event::Event;
use crate::theme::resolve::Theme;

/// A pointer drag in progress: a new event (`id` None), a block being moved, or one being resized.
pub(super) struct WeekDrag {
    pub drag: Drag,
    pub id: Option<u64>,
    /// Whether the pointer may enter the all-day row (a drag that began on the grid mostly may not).
    pub reach: Reach,
}

/// What one render laid out; the pointer is hit-tested against it until the next render.
#[derive(Default)]
pub(super) struct Frame {
    pub days: Vec<NaiveDate>,
    pub events: Vec<Event>,
    /// `events` by id.
    pub index: HashMap<u64, usize>,
    pub layouts: Vec<DayLayout>,
    pub bars: Vec<Bar>,
    pub all_day_h: f32,
}

impl Frame {
    pub fn event(&self, id: u64) -> Option<&Event> {
        self.index.get(&id).map(|&i| &self.events[i])
    }

    /// Whether `id` is on screen: as a block, behind a "+n more", or as an all-day bar.
    pub fn shows(&self, id: u64) -> bool {
        self.layouts.iter().any(|l| l.blocks.iter().any(|b| b.id == id) || l.more.iter().any(|m| m.ids.contains(&id))) || self.bars.iter().any(|b| b.id == id)
    }
}

pub struct WeekView {
    pub(super) scroll: ScrollHandle,
    pub(super) focus: FocusHandle,
    pub(super) drag: Option<WeekDrag>,
    /// The one event the grid's arrow keys have landed on.
    pub(super) roving: Option<u64>,
    pub(super) frame: Rc<Frame>,
    /// The last pointer position of a drag: an edge scroll goes on while it rests.
    pub(super) pointer: Option<Point<Pixels>>,
    /// The days (first, count) the grid was last scrolled for: a new date or a Day/Week switch scrolls again.
    opened: Option<(NaiveDate, usize)>,
}

impl WeekView {
    pub fn new(cx: &mut gpui_kit::App) -> Self {
        Self { scroll: ScrollHandle::new(), focus: cx.focus_handle(), drag: None, roving: None, frame: Rc::default(), pointer: None, opened: None }
    }

    /// Leaving the view: the next visit scrolls to the time of day again.
    pub(super) fn closed(&mut self) {
        self.opened = None;
        self.drag = None;
        self.pointer = None;
    }
}

impl Shell {
    /// The event the grid's arrow keys are on.
    pub fn week_roving(&self) -> Option<u64> {
        self.week.roving
    }

    /// Where the pointer is measured from: the all-day row's top-left. `None` before the first layout.
    pub(super) fn week_area(&self) -> Option<(Area, f32, f32)> {
        let b = self.week.scroll.bounds();
        let all_day_h = self.week.frame.all_day_h;
        (b.size.height > px(0.)).then(|| {
            let area = Area { width: f32::from(b.size.width), view_h: f32::from(b.size.height), all_day_h, scroll: f32::from(-self.week.scroll.offset().y) };
            (area, f32::from(b.origin.x), f32::from(b.origin.y) - all_day_h)
        })
    }

    /// Visible events with an in-progress drag applied so it shows live; a create drag adds its ghost.
    fn week_events(&self) -> Vec<Event> {
        let mut events: Vec<Event> = self.model.visible_events().into_iter().cloned().collect();
        if let Some(w) = &self.week.drag {
            match w.id {
                Some(id) => events.iter_mut().filter(|e| e.id == id).for_each(|e| *e = w.drag.apply(e)),
                None if w.drag.moved() => {
                    let p = w.drag.preview();
                    events.extend(self.model.new_event_calendar().map(|calendar| Event {
                        id: u64::MAX,
                        title: super::model::NEW_EVENT_TITLE.into(),
                        calendar,
                        start: p.start,
                        end: p.end,
                        all_day: p.all_day,
                    }));
                }
                None => {}
            }
        }
        events
    }

    /// After each paint of the grid (its viewport has its real size by then): scroll for newly shown days, to the
    /// current time when today is shown, else 08:00; and keep an edge drag scrolling while the pointer rests.
    fn week_frame(&mut self, cx: &mut Context<Self>) {
        let view_h = f32::from(self.week.scroll.bounds().size.height);
        let days = &self.week.frame.days;
        let Some(&first) = days.first().filter(|_| view_h > 0.) else { return };
        let shown = (first, days.len());
        if self.week.opened != Some(shown) {
            let now = self.model.now;
            let now_min = days.contains(&self.model.today).then(|| (now.hour() * 60 + now.minute()) as i32);
            self.week.scroll.set_offset(point(px(0.), px(-scroll_target(now_min, view_h))));
            self.week.opened = Some(shown);
            cx.notify();
        } else if self.week.pointer.is_some_and(|p| self.week_edge_step(p)) {
            cx.notify();
        }
    }

    pub(super) fn week_view(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let days: Vec<NaiveDate> = self.model.shown_days().days().collect();
        let events = self.week_events();
        let (layouts, bars, lanes) = {
            let refs: Vec<&Event> = events.iter().collect();
            let (bars, lanes) = all_day_bars(days[0], days.len(), &refs);
            (days.iter().map(|&d| layout_day(d, &refs)).collect(), bars, lanes)
        };
        let index = events.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
        let frame = Rc::new(Frame { days, events, index, layouts, bars, all_day_h: all_day_height(lanes) });
        self.week.frame = frame.clone();

        let n = &t.neutral;
        let grid_focused = self.week.focus.is_focused(window);
        let roving = self.week.roving.filter(|&id| grid_focused && frame.shows(id));
        let hour24 = self.model.store.settings.time_format.hour24();
        let weekend = |d: NaiveDate| matches!(d.weekday(), Weekday::Sat | Weekday::Sun);
        let gutter = |inner: AnyElement| div().w(px(GUTTER_W)).flex_none().h_full().relative().child(inner);

        // Day header: weekday in text-3, date in text-1, today as an accent circle.
        let heads: Vec<_> = frame
            .days
            .iter()
            .map(|&d| {
                let today = d == self.model.today;
                let num = div().min_w(px(24.)).h(px(24.)).flex().items_center().justify_center().rounded_full().font_weight(FontWeight::SEMIBOLD);
                let num = if today { num.bg(c(t.accent)).text_color(c(t.accent_on)) } else { num.text_color(c(n.t1)) };
                div()
                    .id(SharedString::from(format!("week-head-{d}")))
                    .test_support()
                    .role(Role::ColumnHeader)
                    .aria_label(d.format("%A, %B %-d").to_string())
                    .flex_1()
                    .min_w_0()
                    .py(px(8.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .child(div().text_size(px(12.)).font_weight(FontWeight::MEDIUM).text_color(c(n.t3)).child(d.format("%a").to_string()))
                    .child(num.child(d.format("%-d").to_string()))
                    .into_any_element()
            })
            .collect();
        let head = div().flex_none().flex().border_t_1().border_color(c(n.line)).child(div().w(px(GUTTER_W)).flex_none()).children(heads);

        let all_day = div()
            .id("week-all-day")
            .test_support()
            .flex_none()
            .flex()
            .h(px(frame.all_day_h))
            .border_t_1()
            .border_b_1()
            .border_color(c(n.line))
            .child(div().w(px(GUTTER_W)).flex_none().pr(px(8.)).pt(px(8.)).flex().justify_end().text_size(px(12.)).text_color(c(n.t3)).child("All day"))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .h_full()
                    .child(div().absolute().inset_0().flex().children(frame.days.iter().map(|&d| div().flex_1().h_full().when(weekend(d), |s| s.bg(c(n.subtle))))))
                    .children(frame.bars.iter().filter_map(|b| self.week_bar(t, &frame, b, roving, hour24))),
            );

        let labels = (0..24).map(|h| {
            let top = if h == 0 { 0. } else { minute_to_y(h * 60) - 8. };
            div().absolute().top(px(top)).right(px(8.)).text_size(px(12.)).text_color(c(n.t3)).child(format_hour(h as u32, hour24))
        });
        let lines = (1..24).map(|h| div().absolute().top(px(h as f32 * HOUR_H)).left_0().right_0().h(px(1.)).bg(c(n.line)));
        let columns = frame.days.iter().enumerate().map(|(ix, &d)| self.week_column(t, &frame, ix, d, roving, hour24));
        let body = div()
            .relative()
            .flex_1()
            .h_full()
            .child(div().absolute().inset_0().flex().children(frame.days.iter().map(|&d| div().flex_1().h_full().when(weekend(d), |s| s.bg(c(n.subtle))))))
            .child(div().absolute().inset_0().children(lines))
            .child(div().absolute().inset_0().flex().children(columns))
            .when(grid_focused && roving.is_none(), |s| s.child(div().id("week-ring").test_support().absolute().inset_0().border_2().border_color(c(t.ring))));
        let grid = div().relative().flex().w_full().h(px(GRID_H)).child(gutter(div().children(labels).into_any_element())).child(body);

        // Window-level listeners: a drag keeps tracking when the pointer leaves the grid. A press is the
        // element's own, so one that lands on an overlay above the grid never starts a drag.
        let this = cx.entity();
        let pointer = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                use gpui_kit::{DispatchPhase, MouseButton, MouseMoveEvent, MouseUpEvent};
                let t = this.clone();
                window.on_mouse_event(move |e: &MouseMoveEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        t.update(cx, |s, cx| s.week_move(e.position, cx));
                    }
                });
                let t = this.clone();
                window.on_mouse_event(move |e: &MouseUpEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble && e.button == MouseButton::Left {
                        t.update(cx, |s, cx| s.week_up(e.position, window, cx));
                    }
                });
            },
        )
        .absolute()
        .size_0();
        // Painted after the grid, so the scroll handle's bounds are this frame's.
        let this = cx.entity();
        let settle = canvas(|_, _, _| (), move |_, _, _, cx| this.update(cx, |s, cx| s.week_frame(cx))).absolute().size_0();

        div()
            .id("week")
            .test_support()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, e: &MouseDownEvent, window, cx| this.week_down(e.position, window, cx)))
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(pointer)
            .child(head)
            .child(all_day)
            .child(
                div()
                    .id("week-scroll")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.week.scroll)
                    // One Tab stop for the whole grid; the arrow keys move inside it.
                    .track_focus(&self.week.focus.clone().tab_stop(true))
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                        if !e.keystroke.modifiers.modified() && this.week_key(&e.keystroke.key, window, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .child(grid),
            )
            .child(settle)
            .into_any_element()
    }

    /// One day's column of the grid: its blocks, "+n more" blocks and, on today, the current-time marker.
    fn week_column(&self, t: &Theme, frame: &Frame, ix: usize, day: NaiveDate, roving: Option<u64>, hour24: bool) -> AnyElement {
        let lay = &frame.layouts[ix];
        let lifted = self.week.drag.as_ref().and_then(|w| w.id.filter(|_| w.drag.moved()));
        let blocks = lay.blocks.iter().filter_map(|b| {
            let e = frame.event(b.id)?;
            Some(self.week_block(t, e, b, day, (roving == Some(b.id), lifted == Some(b.id), hour24)))
        });
        let more = lay.more.iter().map(|m| self.week_more(t, m, day, roving.is_some_and(|r| m.ids.contains(&r))));
        let now = (day == self.model.today).then(|| {
            let y = minute_to_y((self.model.now.hour() * 60 + self.model.now.minute()) as i32);
            div()
                .id("week-now")
                .test_support()
                .role(Role::Status)
                .aria_label(format!("Current time, {}", crate::domain::dates::format_time(self.model.now, hour24)))
                .absolute()
                .top(px(y - 5.))
                .left_0()
                .right_0()
                .h(px(10.))
                .child(div().absolute().top(px(4.)).left_0().right_0().h(px(2.)).bg(c(t.accent)))
                .child(div().absolute().top_0().left(px(-4.)).size(px(10.)).rounded_full().bg(c(t.accent)))
        });
        div().id(SharedString::from(format!("week-col-{day}"))).test_support().flex_1().min_w_0().h_full().relative().children(more).children(blocks).children(now).into_any_element()
    }
}
