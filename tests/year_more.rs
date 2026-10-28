//! Headless tests of the Year view: scroll resets, the focus ring, dot markers, held keys, caching.
mod common;

use common::year::*;
use gpui_calendar::domain::year::{Cursor, DATE_BOX, MAX_COLS, MORE_H};
use gpui_calendar::domain::event::Event;
use gpui_calendar::store::{PERSONAL, SIDE, WORK};
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
    let (h, shell) = open_year_with(cx, "yearm-scroll", (960., 600.), evs());
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

#[gpui_kit::test]
fn all_twelve_months_show_four_across_at_1440_without_scrolling(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-four", 1440.);
    cx.update_window(h, |_, w, _| {
        let view = w.find("year-scroll").bounds();
        let months: Vec<_> = (1..=12).map(|m| w.find(format!("year-month-2026-{m:02}")).bounds()).collect();
        for (i, b) in months.iter().enumerate() {
            assert!(b.top() >= view.top() && b.bottom() <= view.bottom() && b.right() <= view.right(), "month {} lies inside the viewport: {b:?} in {view:?}", i + 1);
            assert_eq!(b.top(), months[i / MAX_COLS * MAX_COLS].top(), "month {} shares its row", i + 1);
            if i % MAX_COLS > 0 {
                assert!(b.left() > months[i - 1].left(), "month {} is right of the one before", i + 1);
            }
        }
        assert!(months[4].top() > months[0].top() && months[8].top() > months[4].top(), "three rows");
        let last_day = w.find("year-day-2026-12-31").bounds();
        assert!(last_day.bottom() <= view.bottom(), "the grid needs no scrolling: {last_day:?} in {view:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_column_count_drops_with_the_width_and_nothing_overflows_sideways(cx: &mut TestAppContext) {
    for (w_px, cols) in [(1440., 4), (960., 3), (700., 2), (520., 1)] {
        let (h, _) = open_year(cx, &format!("year-cols-{w_px}"), w_px);
        cx.update_window(h, |_, w, _| {
            let view = w.find("year-scroll").bounds();
            let tops: Vec<_> = (1..=12).map(|m| w.find(format!("year-month-2026-{m:02}")).bounds()).collect();
            assert_eq!(tops.iter().take_while(|b| b.top() == tops[0].top()).count(), cols, "{w_px}px: months per row");
            assert!(tops.iter().all(|b| b.left() >= view.left() && b.right() <= view.right()), "{w_px}px: no horizontal overflow");
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn dots_are_five_px_right_under_the_day_number(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-dot-size", 1440.);
    cx.update_window(h, |_, w, _| {
        let num = w.find("year-num-2026-10-14").bounds();
        for cal in [WORK, PERSONAL, SIDE] {
            let dot = w.find(format!("year-dot-2026-10-14-{cal}")).bounds();
            assert_eq!((dot.size.width, dot.size.height), (gpui_kit::px(5.), gpui_kit::px(5.)), "dot {cal} is 5px");
            assert!(dot.top() >= num.bottom() && dot.top() - num.bottom() <= gpui_kit::px(1.), "dot {cal} sits directly under the number: {dot:?} vs {num:?}");
        }
    })
    .unwrap();
}

/// Calendars 0..n on October 14, so the day shows three dots and a "+n" marker for the rest.
fn crowded(n: u8) -> Vec<gpui_calendar::domain::event::Event> {
    (0..n).map(|c| timed(c as u64 + 1, "E", c, 14, 9)).collect()
}

/// Asserts the "+n" label lies in its own line box under the date box and inside its day cell.
fn assert_more_fits(w: &mut gpui_kit::Window, hidden_label: &str) {
    let (cell, num, more) = (w.find("year-day-2026-10-14").bounds(), w.find("year-num-2026-10-14").bounds(), w.find("year-more-2026-10-14").bounds());
    assert!(more.top() >= num.bottom(), "the label starts under the date box: {more:?} vs {num:?}");
    assert!(more.bottom() <= cell.bottom() && more.left() >= cell.left() && more.right() <= cell.right(), "the label stays in its day ({hidden_label}): {more:?} in {cell:?}");
    assert!(more.size.height >= px(MORE_H), "own line box: {more:?}");
    let dots = w.find("year-dot-2026-10-14-0").bounds();
    assert!(dots.left() >= cell.left(), "dots stay in the day: {dots:?} in {cell:?}");
}

#[gpui_kit::test]
fn the_plus_marker_has_its_own_line_box_inside_the_day_at_the_height_floor(cx: &mut TestAppContext) {
    let (h, _) = open_year_with(cx, "yearm-more-box", (1440., 769.), crowded(14));
    cx.update_window(h, |_, w, _| {
        assert_more_fits(w, "+2");
        let (cell, num) = (w.find("year-day-2026-10-14").bounds(), w.find("year-num-2026-10-14").bounds());
        assert!(cell.size.height >= px(DATE_BOX + MORE_H), "a day with a marker is tall enough: {cell:?} {num:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_huge_marker_is_capped_and_does_not_spill_into_the_next_day_at_the_month_minimum(cx: &mut TestAppContext) {
    // 1244px is the narrowest window with four 220px months, so a day is about 31px wide.
    let (h, _) = open_year_with(cx, "yearm-more-wide", (1244., 700.), crowded(14));
    cx.update_window(h, |_, w, _| {
        assert_more_fits(w, "9+");
        let (cell, next) = (w.find("year-day-2026-10-14").bounds(), w.find("year-day-2026-10-15").bounds());
        let more = w.find("year-more-2026-10-14").bounds();
        assert!(more.right() <= next.left() && cell.right() <= next.left() + px(0.5), "the marker ends before the next day: {more:?} {next:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn four_columns_that_still_scroll_open_on_the_anchor_month(cx: &mut TestAppContext) {
    let (h, _) = open_year_with(cx, "yearm-four-scroll", (1440., 480.), evs());
    cx.update_window(h, |_, w, _| {
        let (view, jan) = (w.find("year-scroll").bounds(), w.find("year-month-2026-01").bounds());
        assert!(jan.top() < view.top(), "1440x480 needs scrolling with four columns, and the grid is scrolled down to October: {jan:?} in {view:?}");
    })
    .unwrap();
    assert!(oct_on_screen(cx, h), "October, in the third row of four columns, is scrolled to");
}

#[gpui_kit::test]
fn a_resize_that_changes_the_column_count_scrolls_back_to_the_anchor(cx: &mut TestAppContext) {
    let (h, _) = open_year_with(cx, "yearm-resize", (1440., 480.), evs());
    wheel(cx, h, 4000.);
    assert!(!oct_on_screen(cx, h), "the wheel took October away");
    cx.simulate_window_resize(h, gpui_kit::size(px(960.), px(600.)));
    assert!(oct_on_screen(cx, h), "three columns now, and October is back on screen");
}

#[gpui_kit::test]
fn a_crowded_day_does_not_make_the_grid_taller_than_the_window(cx: &mut TestAppContext) {
    // Every month of the October row has a marker, so whichever month is tallest grows.
    let at = |m: u32| d(2026, m, 11).and_hms_opt(9, 0, 0).unwrap();
    let evs = (9..=12).flat_map(|m| (0..14).map(move |c| Event::timed(m as u64 * 100 + c as u64, "N", c, at(m), at(m) + chrono::Duration::hours(1)))).collect();
    let (h, _) = open_year_with(cx, "yearm-more-fit", (1440., 825.), evs);
    cx.update_window(h, |_, w, _| {
        let (view, first, last) = (w.find("year-scroll").bounds(), w.find("year-month-2026-01").bounds(), w.find("year-day-2026-12-31").bounds());
        assert!(first.top() >= view.top() && last.bottom() <= view.bottom(), "fits without scrolling with a marker: {first:?}..{last:?} in {view:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn dots_sit_at_the_same_height_on_days_with_and_without_a_marker(cx: &mut TestAppContext) {
    // Oct 14 has a marker; Oct 15 (same week) has three plain dots.
    let mut evs = crowded(5);
    evs.extend((0..3).map(|c| timed(100 + c as u64, "F", c, 15, 9)));
    let (h, _) = open_year_with(cx, "yearm-dot-align", (1440., 790.), evs);
    cx.update_window(h, |_, w, _| {
        let (marked, plain) = (w.find("year-dot-2026-10-14-0").bounds(), w.find("year-dot-2026-10-15-0").bounds());
        assert_eq!(marked.top(), plain.top(), "dots align across the week: {marked:?} vs {plain:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn every_month_shows_six_week_rows_and_the_padding_days_are_not_buttons(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-six-rows", 1440.);
    cx.update_window(h, |_, w, _| {
        // February 2026 starts on a Sunday: Jan 26 to Mar 1 is five rows, and Mar 2 to 8 is a sixth of padding.
        let (feb_2, feb_28) = (w.find("year-day-2026-02-02").bounds(), w.find("year-day-2026-02-28").bounds());
        let (pad_first, pad_last) = (w.find("year-pad-02-2026-03-02").bounds(), w.find("year-pad-02-2026-03-08").bounds());
        assert!(pad_first.top() > feb_28.top() && pad_first.left() == feb_2.left(), "the padding starts a new row under the 28th: {pad_first:?}");
        assert!(pad_last.top() == pad_first.top() && pad_last.left() > pad_first.left(), "and fills that row");
        // Every month ends on the same row count, so each month block is the same height: Dec's last pad is 5 rows under its first row.
        let (jan_1, jan_pad) = (w.find("year-day-2026-01-01").bounds(), w.find("year-pad-01-2026-02-08").bounds());
        assert!(jan_pad.bottom() - jan_1.top() > feb_2.size.height * 5., "six rows tall: {jan_1:?} to {jan_pad:?}");
        assert!(w.try_find("year-pad-01-2026-01-31").is_none(), "an in-month day is a button, not a pad");
        let view = w.find("year-scroll").bounds();
        assert!(w.find("year-pad-12-2027-01-10").bounds().bottom() <= view.bottom(), "six rows still fit at 1440x900 without scrolling");
    })
    .unwrap();
}
