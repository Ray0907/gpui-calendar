//! Year view: twelve mini months with event dots. A day drills to the Day view, a month title to the
//! Month view. The pure parts (grids, marks, columns, arrow steps) live in `domain::year`.

mod cache;
mod input;

use std::cell::Cell;
use std::rc::Rc;

use chrono::{Datelike, NaiveDate, Weekday};
use gpui_kit::{
    AnyElement, Bounds, ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    Role, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px, prelude::*,
};

use super::controls::{c, ring, shape_of, shape_sized};
use super::shell::{CHROME_H as SHELL_CHROME_H, Shell};
use crate::domain::month::grid_width;
use crate::domain::year::{Cursor, DATE_BOX, DOT_GAP, DOT_PX, DayMarks, MAX_COLS, MONTH_GAP, MORE_H, MonthGrid, columns_for, more_label, row_height, cursor_in, day_height, month_at, month_cell};
use cache::{Cache, Key};
use crate::theme::resolve::Theme;

/// Padding around the grid, and the gap between months.
const PAD: f32 = 16.;
const GAP: f32 = MONTH_GAP;
/// What the window spends above and below the days: the shell's chrome and the grid's padding.
const CHROME_H: f32 = SHELL_CHROME_H + 2. * PAD;
/// A month's title (28) and weekday row (18).
const MONTH_HEAD_H: f32 = 28. + 18.;

pub struct YearView {
    pub(super) focus: FocusHandle,
    pub(super) scroll: ScrollHandle,
    /// Month columns of the last render: the scroll target of an arrow step.
    pub(super) cols: usize,
    /// The reset count (`Model::year_epoch`) and column count the scroll was last placed for: every reset (Today, Prev/Next,
    /// a view change, a resize to another column count) brings the anchor's month on screen, whether or not the anchor moved.
    placed: Option<(u32, usize)>,
    cache: Option<Rc<Cache>>,
    builds: usize,
    /// Day buttons built by the last frame: one per day of the year, whatever the grids show twice.
    buttons: Cell<usize>,
}

impl YearView {
    pub fn new(cx: &mut gpui_kit::App) -> Self {
        Self { focus: cx.focus_handle(), scroll: ScrollHandle::new(), cols: MAX_COLS, placed: None, cache: None, builds: 0, buttons: Cell::new(0) }
    }
}

impl Shell {
    /// How often the Year view rebuilt its grids and marks (it should not on a frame where nothing changed).
    pub fn year_builds(&self) -> usize {
        self.year.builds
    }

    /// Day buttons the last Year frame built: the filler days of a month are not among them.
    pub fn year_day_buttons(&self) -> usize {
        self.year.buttons.get()
    }

    pub(super) fn year_view(&mut self, t: &Theme, narrow: bool, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let year = self.model.shown_year();
        let width = grid_width(f32::from(window.viewport_size().width), narrow) - 2. * PAD;
        self.year.cols = columns_for(width);
        let cols = self.year.cols;
        // Arrow keys scroll to their own stop; after a reset the anchor's month is the one to show.
        let epoch = self.model.year_epoch();
        if self.year.placed != Some((epoch, cols)) {
            self.year.placed = Some((epoch, cols));
            if self.model.year_cursor.is_none() {
                self.year.scroll.scroll_to_top_of_item(month_cell(self.model.anchor.month(), cols).0);
            }
        }
        self.year.buttons.set(0);
        let events = self.model.visible_events();
        let key = Key::new(year, self.model.week_start(), &events);
        let cache = match &self.year.cache {
            Some(c) if c.key == key => c.clone(),
            _ => {
                let c = Rc::new(Cache::build(key, &events));
                self.year.builds += 1;
                self.year.cache = Some(c.clone());
                c
            }
        };
        let focused = self.year.focus.is_focused(window);
        // Leaving the grid drops its roving stop: a stale one would pin the grid where the keys left it.
        if !focused {
            self.model.year_cursor = None;
        }
        let cursor = focused.then(|| cursor_in(self.model.year_cursor, self.model.anchor, year));
        // The cursor's month row scrolled out of view: say where the focus is.
        let hint = cursor.filter(|&c| self.year_scrolled_away(c)).map(|c| match c {
            Cursor::Day(d) => d.format("%A, %B %-d").to_string(),
            Cursor::Title(y, m) => NaiveDate::from_ymd_opt(y, m, 1).expect("valid month").format("%B").to_string(),
        });
        let weekdays = self.model.mini_weekdays();
        let grids = &cache.grids;
        // Week rows stacked down the page (the tallest month of each row): the day height fills the window.
        let nrows = 12usize.div_ceil(cols);
        let stacked: usize = (0..nrows).map(|r| (0..cols).filter_map(|col| month_at(r, col, cols)).map(|m| grids[m as usize - 1].rows().count()).max().unwrap_or(0)).sum();
        // Marker weeks grow by at most this much each (at the shortest day); the tallest month of each row bounds it.
        let grow: usize = (0..nrows).map(|r| (0..cols).filter_map(|col| month_at(r, col, cols)).map(|m| grids[m as usize - 1].marked_weeks(&cache.marks).into_iter().filter(|x| *x).count()).max().unwrap_or(0)).sum();
        let grow = grow as f32 * (row_height(DATE_BOX + DOT_PX, true) - DATE_BOX - DOT_PX);
        let avail = f32::from(window.viewport_size().height) - CHROME_H - nrows as f32 * (MONTH_HEAD_H + GAP) + GAP - grow;
        let day_h = day_height(avail, stacked);
        let rows: Vec<_> = (0..nrows)
            .map(|r| {
                let months = (0..cols).map(|col| match month_at(r, col, cols) {
                    Some(m) => self.year_month(t, year, &cache, &grids[m as usize - 1], (cursor, day_h), &weekdays, cx),
                    None => div().flex_1().min_w_0().into_any_element(),
                });
                div().flex().gap(px(GAP)).children(months).into_any_element()
            })
            .collect();
        div()
            .id("year")
            .test_support()
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .children(hint.map(|h| {
                div()
                    .id("year-focus-hint")
                    .test_support()
                    .absolute()
                    .bottom(px(PAD))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(div().px(px(12.)).py(px(6.)).rounded_full().bg(c(t.neutral.t1)).text_color(c(t.neutral.surface)).text_size(px(12.)).font_weight(FontWeight::MEDIUM).child(format!("Focus: {h}")))
            }))
            .child(
                div()
                    .id("year-scroll")
                    .test_support()
                    .role(Role::Group)
                    .aria_label("Year grid")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.year.scroll)
                    // One Tab stop for the whole grid; the arrow keys move inside it.
                    .track_focus(&self.year.focus.clone().tab_stop(true))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, window, cx| this.year.focus.focus(window, cx)))
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                        if !e.keystroke.modifiers.modified() && this.year_key(&e.keystroke.key, e.is_held, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .p(px(PAD))
                    .flex()
                    .flex_col()
                    .gap(px(GAP))
                    .children(rows),
            )
            .into_any_element()
    }

    /// `true` when the row of months holding the cursor is not on screen (by the last frame's layout).
    fn year_scrolled_away(&self, c: Cursor) -> bool {
        let month = c.month();
        let view = self.year.scroll.bounds();
        // The recorded item bounds do not include the scroll offset.
        let at = self.year.scroll.offset();
        self.year.scroll.bounds_for_item(month_cell(month, self.year.cols).0).is_some_and(|b| !Bounds { origin: b.origin + at, size: b.size }.intersects(&view))
    }

    #[allow(clippy::too_many_arguments)]
    fn year_month(&self, t: &Theme, year: i32, cache: &Cache, g: &MonthGrid, (cursor, day_h): (Option<Cursor>, f32), weekdays: &[String], cx: &Context<Self>) -> AnyElement {
        let n = &t.neutral;
        let month = g.month;
        let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid month");
        let title = div()
            .id(SharedString::from(format!("year-month-{year}-{month:02}")))
            .test_support()
            .role(Role::Button)
            .aria_label(format!("{}, open in Month view", first.format("%B %Y")))
            .relative()
            .h(px(28.))
            .px(px(4.))
            .flex()
            .items_center()
            .rounded(px(6.))
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .active(|s| s.opacity(0.85))
            .when(cursor == Some(Cursor::Title(year, month)), |s| s.child(self.year_ring(t, format!("year-ring-month-{year}-{month:02}"))))
            .child(first.format("%B").to_string())
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.model.drill_month(year, month);
                this.sync(cx);
            }));
        let head = div().flex().children(weekdays.iter().map(|w| {
            div().flex_1().h(px(18.)).flex().items_center().justify_center().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(c(n.t3)).child(w.to_uppercase())
        }));
        let marked = g.marked_weeks(&cache.marks);
        let rows = g.rows().zip(marked).map(|(row, has_more)| {
            let day_h = row_height(day_h, has_more);
            let days = row.iter().map(|&d| if g.in_month(d) { self.year_day(t, d, cache, cursor == Some(Cursor::Day(d)), day_h, cx) } else { self.year_pad(t, d, day_h) });
            div().flex().children(days)
        });
        div().flex_1().min_w_0().flex().flex_col().child(title).child(head).children(rows).into_any_element()
    }

    /// A date outside the month: shown dim for the week shape. It has no id, role or name: the real
    /// button is in its own month.
    fn year_pad(&self, t: &Theme, d: NaiveDate, day_h: f32) -> AnyElement {
        let weekend = is_weekend(d);
        let num = div().h(px(DATE_BOX)).flex().items_center().text_size(px(12.)).font_weight(FontWeight::MEDIUM).text_color(c(t.neutral.t3)).child(d.format("%-d").to_string());
        div().flex_1().min_w_0().h(px(day_h)).flex().flex_col().items_center().rounded(px(6.)).when(weekend, |s| s.bg(c(t.neutral.subtle))).child(num).into_any_element()
    }

    /// The focus ring of the roving stop: an element of its own over the whole day or title, so it never
    /// reads as the today circle, and a headless test can tell where it is. It paints first, so the surface
    /// fill that keeps the ring's shadow out of the interior sits under the content.
    fn year_ring(&self, t: &Theme, id: String) -> AnyElement {
        div().id(SharedString::from(id)).test_support().absolute().inset_0().rounded(px(6.)).bg(c(t.neutral.surface)).shadow(ring(c(t.neutral.surface), c(t.ring))).into_any_element()
    }

    fn year_day(&self, t: &Theme, date: NaiveDate, cache: &Cache, cursor: bool, day_h: f32, cx: &Context<Self>) -> AnyElement {
        let n = &t.neutral;
        let today = date == self.model.today;
        let marks = cache.marks.get(&date);
        self.year.buttons.set(self.year.buttons.get() + 1);
        let num = div()
            .id(SharedString::from(format!("year-num-{date}")))
            .test_support()
            .min_w(px(DATE_BOX))
            .h(px(DATE_BOX))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .text_size(px(12.))
            .child(date.format("%-d").to_string());
        let num = if today { num.bg(c(t.accent)).text_color(c(t.accent_on)).font_weight(FontWeight::SEMIBOLD) } else { num.text_color(c(n.t1)).font_weight(FontWeight::MEDIUM) };
        // One dot per calendar, in the calendar's shape, so identity never rests on color alone; a calendar
        // without a theme gets a neutral dot. Only calendars past the third are counted in a "+n".
        let (shown, hidden) = marks.map_or((&[][..], 0), DayMarks::dots);
        let dots = div()
            .h(px(DOT_PX))
            .flex()
            .items_start()
            .gap(px(DOT_GAP))
            .children(shown.iter().map(|&id| {
                let dot = shape_sized(shape_of(id), t.cal(id).map_or(c(n.t3), |k| c(k.dot)), DOT_PX);
                div().id(SharedString::from(format!("year-dot-{date}-{id}"))).test_support().child(dot)
            }))
            .when(hidden > 0, |s| {
                s.child(div().id(SharedString::from(format!("year-more-{date}"))).test_support().flex_none().text_size(px(8.)).line_height(px(MORE_H)).text_color(c(n.t2)).child(more_label(hidden)))
            });
        let cell = div()
            .id(SharedString::from(format!("year-day-{date}")))
            .test_support()
            .role(Role::Button)
            .aria_label(cache.names[&date].clone())
            .relative()
            .flex_1()
            .min_w_0()
            .h(px(day_h))
            .flex()
            .flex_col()
            .items_center()
            .rounded(px(6.))
            .cursor_pointer()
            .active(|s| s.opacity(0.85))
            .when(is_weekend(date), |s| s.bg(c(n.subtle)))
            .when(cursor, |s| s.child(self.year_ring(t, format!("year-ring-{date}"))))
            .child(num)
            .child(dots)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.model.drill_day(date);
                this.sync(cx);
            }));
        if today { cell.aria_description("current date").into_any_element() } else { cell.into_any_element() }
    }
}

fn is_weekend(d: NaiveDate) -> bool {
    matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}
