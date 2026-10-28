//! Headless tests of the Year view: scroll resets, the focus ring, dot markers, held keys, caching.
mod common;

use common::year::*;
use gpui_calendar::domain::year::Cursor;
use gpui_calendar::store::{PERSONAL, WORK};
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AppContext, TestAppContext, point, px};

fn wheel(cx: &mut TestAppContext, h: gpui_kit::AnyWindowHandle, dy: f32) {
    cx.update_window(h, |_, w, cx| {
        w.scroll("year-scroll", gpui_kit::ScrollDelta::Pixels(point(px(0.), px(dy))), cx);
        (0..2).for_each(|_| w.render_frame(cx));
    })
    .unwrap();
}

/// Whether October 28's date box is inside the scroll area (two frames to settle).
fn oct_on_screen(cx: &mut TestAppContext, h: gpui_kit::AnyWindowHandle) -> bool {
    cx.update_window(h, |_, w, cx| {
        (0..2).for_each(|_| w.render_frame(cx));
        let (view, num) = (w.find("year-scroll").bounds(), w.find("year-num-2026-10-28").bounds());
        num.top() >= view.top() && num.bottom() <= view.bottom()
    })
    .unwrap()
}

#[gpui_kit::test]
fn today_and_view_changes_scroll_back_to_the_anchor_even_when_it_did_not_move(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "yearm-scroll", 1440.);
    assert!(oct_on_screen(cx, h));
    // The wheel moves the grid without a roving day, and the anchor stays on Oct 28.
    wheel(cx, h, 4000.);
    assert!(!oct_on_screen(cx, h), "the wheel took October away");
    cx.update_window(h, |_, w, cx| w.click("nav-today", cx)).unwrap();
    assert!(oct_on_screen(cx, h), "Today on an unchanged anchor brings October back");
    wheel(cx, h, 4000.);
    assert!(!oct_on_screen(cx, h));
    cx.update_window(h, |_, w, cx| {
        w.click("view-month", cx);
        w.click("view-year", cx);
    })
    .unwrap();
    assert!(oct_on_screen(cx, h), "leaving and returning to Year starts on the anchor's month");
    wheel(cx, h, 4000.);
    cx.update_window(h, |_, w, cx| {
        w.click("nav-next", cx);
        w.click("nav-prev", cx);
    })
    .unwrap();
    assert!(oct_on_screen(cx, h), "Next then Prev lands on October again");
    cx.update(|cx| assert_eq!(shell.read(cx).model.anchor, d(2026, 10, 28)));
}

#[gpui_kit::test]
fn the_focus_ring_is_drawn_on_the_roving_day_and_the_title_and_only_while_focused(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "yearm-ring", 1440.);
    assert!(!ringed(cx, h, "year-day-2026-10-28"), "no ring while the grid is not focused");
    focus_grid(cx, h);
    assert!(ringed(cx, h, "year-day-2026-10-28"));
    cx.update_window(h, |_, w, _| {
        let (ring, day) = (w.find("year-ring-2026-10-28").bounds(), w.find("year-day-2026-10-28").bounds());
        assert_eq!(ring, day, "the ring is on the whole day");
        assert_eq!(w.find("year-day-2026-10-28").selected(), None, "a button carries no selected state");
        assert_eq!(w.find("year-month-2026-10").selected(), None);
    })
    .unwrap();
    keys(cx, h, &["right"]);
    assert!(ringed(cx, h, "year-day-2026-10-29") && !ringed(cx, h, "year-day-2026-10-28"), "the ring moves with the stop");
    keys(cx, h, &["up"; 5]);
    assert!(ringed(cx, h, "year-month-2026-10"), "and sits on a month title");
    cx.update_window(h, |_, w, _| assert_eq!(w.find("year-ring-month-2026-10").bounds(), w.find("year-month-2026-10").bounds())).unwrap();
}

#[gpui_kit::test]
fn leaving_the_grid_drops_the_roving_stop_and_its_ring(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "yearm-blur", 1440.);
    focus_grid(cx, h);
    keys(cx, h, &["right", "right"]);
    cx.update(|cx| assert!(shell.read(cx).model.year_cursor.is_some()));
    cx.update_window(h, |_, w, cx| {
        let at = w.find("calendar-row-0").bounds().center();
        w.pointer_down(at, gpui_kit::MouseButton::Left, cx);
        w.render_frame(cx);
        assert_eq!(w.find("year-scroll").focused(), Some(false), "the press moved focus off the grid");
    })
    .unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.year_cursor, None, "blur clears it"));
    assert!(!ringed(cx, h, "year-day-2026-10-30"));
}

#[gpui_kit::test]
fn three_calendars_show_three_dots_and_no_marker_even_when_one_has_no_theme(cx: &mut TestAppContext) {
    let events = vec![timed(1, "A", WORK, 14, 9), timed(2, "B", PERSONAL, 14, 10), timed(3, "C", 3, 14, 11), timed(4, "D", WORK, 15, 9), timed(5, "E", PERSONAL, 15, 10), timed(6, "F", 2, 15, 11), timed(7, "G", 3, 15, 12)];
    let (h, _) = open_year_with(cx, "yearm-plus", (1440., 900.), events);
    cx.update_window(h, |_, w, _| {
        for cal in [WORK, PERSONAL, 3] {
            assert!(w.try_find(format!("year-dot-2026-10-14-{cal}")).is_some(), "calendar {cal} has a dot on the 14th");
        }
        assert!(w.try_find("year-more-2026-10-14").is_none(), "three calendars: no marker, themed or not");
        assert!(w.try_find("year-more-2026-10-15").is_some(), "four calendars: a marker");
        assert!(w.try_find("year-dot-2026-10-15-3").is_none(), "the fourth calendar is the marker, not a dot");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_held_enter_opens_nothing_and_a_press_opens_once(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "yearm-held", 1440.);
    focus_grid(cx, h);
    cx.update_window(h, |_, w, cx| w.key_down("enter", true, cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.view, View::Year, "an auto-repeat does not drill"));
    cx.update_window(h, |_, w, cx| w.key_down("space", true, cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.view, View::Year));
    cx.update_window(h, |_, w, cx| w.key_down("right", true, cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.year_cursor, Some(Cursor::Day(d(2026, 10, 29))), "a held arrow keeps walking"));
    cx.update_window(h, |_, w, cx| w.key_down("enter", false, cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.view, View::Day));
}

#[gpui_kit::test]
fn a_new_first_weekday_resets_the_roving_stop(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "yearm-week-start", 1440.);
    focus_grid(cx, h);
    keys(cx, h, &["right"]);
    cx.update(|cx| shell.update(cx, |s, cx| {
        assert!(s.model.year_cursor.is_some());
        s.model.set_week_start(chrono::Weekday::Sun);
        assert_eq!(s.model.year_cursor, None);
        s.sync(cx);
    }));
}

#[gpui_kit::test]
fn grids_and_marks_are_rebuilt_only_when_their_inputs_change(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "yearm-cache", 1440.);
    let builds = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).year_builds());
    let first = builds(cx);
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    focus_grid(cx, h);
    keys(cx, h, &["right", "down"]);
    assert_eq!(builds(cx), first, "frames, focus and arrow keys reuse the build");
    cx.update(|cx| shell.update(cx, |s, cx| {
        s.model.toggle_calendar(WORK);
        s.sync(cx);
    }));
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
    assert_eq!(builds(cx), first + 1, "a hidden calendar changes the marks");
    cx.update_window(h, |_, w, cx| w.click("nav-next", cx)).unwrap();
    assert_eq!(builds(cx), first + 2, "another year changes the grids");
}

#[gpui_kit::test]
fn modified_arrows_and_enter_are_left_alone_in_the_grid(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "yearm-mods", 1440.);
    focus_grid(cx, h);
    cx.update_window(h, |_, w, cx| {
        for chord in ["cmd-right", "ctrl-right", "alt-down", "shift-left", "cmd-enter", "ctrl-enter", "alt-enter", "shift-enter"] {
            w.press(chord, cx);
        }
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(shell.read(cx).model.year_cursor, None, "no chord moved the roving day");
        assert_eq!(shell.read(cx).model.view, View::Year, "no chord drilled");
    });
    cx.update_window(h, |_, w, cx| w.press("right", cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.year_cursor, Some(Cursor::Day(d(2026, 10, 29))), "the plain arrow still does"));
}
