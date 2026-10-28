//! Headless month view tests: clamped navigation, hidden rows, rows per width, hover cost.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::domain::event::Event;
use gpui_calendar::store::WORK;
use gpui_calendar::ui::Shell;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, ScrollDelta, TestAppContext, point, px};
use std::{cell::Cell, rc::Rc};

const WEEK_H: f32 = 128.;

/// Six half-hour events on Wed Oct 14, ids 100 to 105.
fn busy_day(s: &mut Shell) {
    for i in 0..6u32 {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(8 + i, 0, 0).unwrap();
        s.model.store.events.push(Event::timed(100 + u64::from(i), &format!("Busy {i}"), WORK, start, start + chrono::Duration::minutes(30)));
    }
}

fn open_month(cx: &mut TestAppContext, name: &str, size: (f32, f32), setup: impl FnOnce(&mut Shell) + 'static) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), size, Rc::new(Cell::new(at(28, 17, 35))), setup);
    settle(cx, h);
    (h, shell)
}

fn settle(cx: &mut TestAppContext, h: AnyWindowHandle) {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.render_frame(cx);
    })
    .unwrap();
}

fn title(cx: &mut TestAppContext, h: AnyWindowHandle) -> String {
    settle(cx, h);
    cx.update_window(h, |_, w, _| w.find("header-title").label().unwrap().to_string()).unwrap()
}

#[gpui_kit::test]
fn month_far_beyond_the_run_keeps_the_anchor_and_does_not_skip_months(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-far", (1280., 900.), |_| {});
    // The run ends in October 2036; 130 months on is past it.
    cx.update_window(h, |_, w, cx| (0..130).for_each(|_| w.click("nav-next", cx))).unwrap();
    let settled: Vec<String> = (0..4).map(|_| title(cx, h)).collect();
    assert!(settled.iter().all(|t| t == "August 2037"), "the list rests at the run's end, the anchor stays: {settled:?}");
    cx.update(|cx| assert_eq!(shell.read(cx).model.mini_title(), "August 2037", "the mini month agrees"));
    cx.update_window(h, |_, w, cx| w.click("nav-prev", cx)).unwrap();
    assert_eq!(title(cx, h), "July 2037", "one step back is one month back");
}

#[gpui_kit::test]
fn month_rows_follow_the_grid_width_not_the_window(cx: &mut TestAppContext) {
    // 1100 wide has a rail: its grid is as roomy as a 1272 window with a sidebar. 700 is a narrow grid.
    for (w_px, hidden, name) in [(1100., "+3 more", "month-cap-rail"), (1280., "+3 more", "month-cap-side"), (700., "+4 more", "month-cap-narrow")] {
        let (h, _) = open_month(cx, name, (w_px, 900.), busy_day);
        cx.update_window(h, |_, w, _| assert!(w.try_find_by_label(hidden).is_some(), "{w_px}: {hidden} for six events")).unwrap();
    }
}

#[gpui_kit::test]
fn month_a_roving_event_hidden_in_a_count_rings_the_count(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-hidden-ring", (1280., 900.), busy_day);
    cx.update_window(h, |_, w, cx| {
        w.click("month-cell-2026-10-06", cx);
        w.press("right", cx); // Busy 0
        w.press("right", cx); // Busy 1
        w.render_frame(cx);
        assert_eq!(w.find_by_label("+3 more").selected(), Some(false), "a shown event holds the ring itself");
    })
    .unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).month_roving()), Some(101));
    cx.update_window(h, |_, w, cx| {
        w.press("right", cx); // Busy 2: the last shown
        w.press("right", cx); // Busy 3: first hidden
        w.render_frame(cx);
        assert_eq!(w.find_by_label("+3 more").selected(), Some(true), "the count carries the ring");
        w.press("right", cx);
        w.press("right", cx);
        w.press("right", cx); // past the day's events: Sprint planning
        w.render_frame(cx);
        assert_eq!(w.find_by_label("+3 more").selected(), Some(false));
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_arrowing_to_a_partly_cut_week_scrolls_it_whole_into_view(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-cut", (1280., 900.), busy_day);
    cx.update_window(h, |_, w, cx| {
        // Oct 12's week (518) is 2 rows below the October grid's first; leave it half cut at the top.
        w.scroll("month-scroll", ScrollDelta::Pixels(point(px(0.), px(-(2. * WEEK_H + 64.)))), cx);
        w.render_frame(cx);
        w.click("month-cell-2026-10-20", cx);
        w.press("right", cx); // Busy 0, in the cut week
        w.render_frame(cx);
        w.render_frame(cx);
        let (chip, grid) = (w.find("month-ev-100-518").bounds(), w.find("month-scroll").bounds());
        assert!(chip.top() >= grid.top() && chip.bottom() <= grid.bottom(), "{chip:?} inside {grid:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_moving_inside_one_cell_while_dragging_does_not_redraw(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-hover", (1280., 900.), |_| {});
    let redraws = Rc::new(Cell::new(0));
    let seen = redraws.clone();
    let _sub = cx.update(|cx| cx.observe(&shell, move |_, _| seen.set(seen.get() + 1)));
    cx.update_window(h, |_, w, cx| {
        let cell = w.find("month-cell-2026-10-06").bounds();
        let (a, b) = (cell.center(), w.find("month-cell-2026-10-07").bounds().center());
        w.pointer_down(a, MouseButton::Left, cx);
        w.pointer_move(b, Some(MouseButton::Left), cx);
    })
    .unwrap();
    let after_entering = redraws.get();
    assert!(after_entering > 0, "entering another cell redraws");
    cx.update_window(h, |_, w, cx| {
        let cell = w.find("month-cell-2026-10-07").bounds();
        for dx in [-8., -3., 3., 8.] {
            w.pointer_move(cell.center() + point(px(dx), px(0.)), Some(MouseButton::Left), cx);
        }
    })
    .unwrap();
    assert_eq!(redraws.get(), after_entering, "four moves inside the same cell");
}
