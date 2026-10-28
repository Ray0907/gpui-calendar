//! Headless tests of what the week and day views show: filtering, the current-time marker, scroll on
//! open, titles, names, keys, and the cost of a drag with 50 events.
mod common;

use std::{cell::Cell, rc::Rc, time::Instant};

use chrono::NaiveDate;
use common::{at, open_sized, open_week, tmp};
use gpui_calendar::domain::event::Event;
use gpui_calendar::store::{PERSONAL, SIDE, WORK};
use gpui_calendar::ui::model::View;
use gpui_calendar::ui::{BANNER_EYEBROW, BANNER_META_LINE_OF, BANNER_TITLE, DATE_CIRCLE, HEAD_H, LANE_H, line_for, lane_top};
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AppContext, MouseButton, TestAppContext, px};

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

fn timed(id: u64, title: &str, cal: u8, day: u32, h: u32, m: u32, mins: i64) -> Event {
    Event::timed(id, title, cal, at(day, h, m), at(day, h, m) + chrono::Duration::minutes(mins))
}

fn title(cx: &mut TestAppContext, h: gpui_kit::AnyWindowHandle) -> String {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.find("header-title").label().unwrap().to_string()
    })
    .unwrap()
}

#[gpui_kit::test]
fn week_header_shows_the_range_and_prev_next_today_step_by_week(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "week-title", View::Week, vec![]);
    assert_eq!(title(cx, h), "Oct – Nov 2026");
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("view-week").checked(), Some(true));
        assert!(w.try_find("nav-next").is_some_and(|n| n.label() == Some("Next week")));
        w.click("nav-next", cx);
    })
    .unwrap();
    assert_eq!(title(cx, h), "November 2026");
    cx.update_window(h, |_, w, cx| {
        assert!(w.try_find("week-col-2026-11-02").is_some(), "the grid moved to Nov 2 to 8");
        w.click("nav-prev", cx);
        w.click("nav-prev", cx);
    })
    .unwrap();
    assert_eq!(title(cx, h), "October 2026");
    cx.update_window(h, |_, w, cx| w.click("nav-today", cx)).unwrap();
    assert_eq!(title(cx, h), "Oct – Nov 2026");
    cx.update_window(h, |_, w, cx| {
        w.click("view-day", cx);
        w.render_frame(cx);
        assert_eq!(w.find("header-title").label().unwrap(), "Wed 28 Oct 2026", "Day view names the day");
        assert!(w.try_find("week-col-2026-10-28").is_some() && w.try_find("week-col-2026-10-29").is_none());
        assert_eq!(w.find("nav-next").label(), Some("Next day"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_has_named_day_headers_hour_labels_and_a_today_marker(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "week-chrome", View::Week, vec![]);
    cx.update_window(h, |_, w, _| {
        assert!(w.try_find_by_label("Wednesday, October 28").is_some());
        assert_eq!(w.find("week-head-2026-10-28").bounds().size.width, w.find("week-col-2026-10-28").bounds().size.width, "headers sit over their columns");
        assert!(w.try_find_by_label("Current time, 17:35").is_some());
        assert!(w.try_find("week-all-day").is_some());
        for hour in ["09:00", "17:00"] {
            assert!(w.try_find_by_label(hour).is_none(), "hour labels are text, not controls: {hour}");
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_current_time_marker_sits_on_the_clock_and_moves_with_it(cx: &mut TestAppContext) {
    let now = Rc::new(Cell::new(at(28, 17, 35)));
    let (h, shell) = open_sized(cx, &tmp("week-now"), (1440., 900.), now, |s| s.model.set_view(View::Week));
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    let marker_y = |cx: &mut TestAppContext| {
        cx.update_window(h, |_, w, cx| {
            w.render_frame(cx);
            let (m, c) = (w.find("week-now").bounds(), w.find("week-col-2026-10-28").bounds());
            f32::from(m.center().y - c.top())
        })
        .unwrap()
    };
    let before = marker_y(cx);
    assert!((before - (17. * 60. + 35.) * 44. / 60.).abs() < 1.5, "17:35 is {before}px down");
    cx.update(|cx| shell.update(cx, |s, cx| {
        s.model.set_now(at(28, 18, 5));
        cx.notify();
    }));
    assert!((marker_y(cx) - before - 22.).abs() < 0.5, "30 minutes later is 22px lower");
    cx.update_window(h, |_, w, _| assert!(w.try_find_by_label("Current time, 18:05").is_some())).unwrap();
}

#[gpui_kit::test]
fn week_opens_scrolled_to_now_or_to_eight(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "week-scroll-now", View::Week, vec![]);
    cx.update_window(h, |_, w, _| {
        let (view, marker) = (w.find("week-scroll").bounds(), w.find("week-now").bounds());
        assert!(marker.top() >= view.top() && marker.bottom() <= view.bottom(), "17:35 is on screen when opening on today");
    })
    .unwrap();
    let (h, _) = open_sized(cx, &tmp("week-scroll-8"), (1440., 700.), Rc::new(Cell::new(at(28, 17, 35))), |s| {
        s.model.pick_day(NaiveDate::from_ymd_opt(2026, 11, 10).unwrap());
        s.model.set_view(View::Week);
    });
    cx.update_window(h, |_, w, cx| {
        (0..3).for_each(|_| w.render_frame(cx));
        let (view, col) = (w.find("week-scroll").bounds(), w.find("week-col-2026-11-10").bounds());
        assert_eq!(view.top() - col.top(), px(8. * 44.), "another week opens with 08:00 at the top");
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_hiding_a_calendar_removes_its_blocks_and_pills(cx: &mut TestAppContext) {
    let evs = vec![timed(1, "Standup", WORK, 28, 11, 0, 30), timed(2, "Lunch", PERSONAL, 28, 12, 30, 60), Event::all_day(3, "Offsite", WORK, d(29), d(30))];
    let (h, _) = open_week(cx, "week-hidden", View::Week, evs);
    cx.update_window(h, |_, w, cx| {
        for l in ["Standup, 11:00 to 11:30, Work", "Lunch, 12:30 to 13:30, Personal", "Offsite, Work, all day, Oct 29 to Oct 30"] {
            assert!(w.try_find_by_label(l).is_some(), "{l}");
        }
        w.click(format!("calendar-row-{WORK}"), cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("Standup, 11:00 to 11:30, Work").is_none());
        assert!(w.try_find_by_label("Offsite, Work, all day, Oct 29 to Oct 30").is_none());
        assert!(w.try_find_by_label("Lunch, 12:30 to 13:30, Personal").is_some(), "other calendars stay");
        w.click(format!("calendar-row-{WORK}"), cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("Standup, 11:00 to 11:30, Work").is_some(), "and back");
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_overlaps_sit_side_by_side_and_a_third_collapses(cx: &mut TestAppContext) {
    let evs = vec![timed(1, "A", WORK, 28, 11, 0, 60), timed(2, "B", PERSONAL, 28, 11, 15, 60), timed(3, "C", SIDE, 28, 11, 30, 60), timed(4, "Solo", WORK, 29, 11, 0, 60)];
    let (h, _) = open_week(cx, "week-overlap", View::Week, evs);
    cx.update_window(h, |_, w, _| {
        let col = w.find("week-col-2026-10-28").bounds();
        let (a, b) = (w.find_by_label("A, 11:00 to 12:00, Work").bounds(), w.find_by_label("B, 11:15 to 12:15, Personal").bounds());
        let chip = w.find_by_label("+1 more").bounds();
        assert!(a.right() <= b.left() + px(7.), "A and B do not overlap sideways");
        assert!((a.size.width - col.size.width * 0.375).abs() < px(8.), "each takes half of the column left of the chip strip");
        assert!(!a.intersects(&chip) && !b.intersects(&chip), "the chip covers no event");
        assert!(chip.left() >= b.right() - px(7.) && chip.right() <= col.right() + px(1.), "it sits in the strip at the right");
        assert!(w.try_find_by_label("C, 11:30 to 12:30, Side project").is_none(), "the third overlap is not drawn");
        assert!(w.try_find_by_label("+1 more").is_some());
        let solo = w.find_by_label("Solo, 11:00 to 12:00, Work").bounds();
        assert!(solo.size.width > col.size.width - px(8.), "a lone event takes the whole column");
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_arrow_keys_walk_the_events_in_reading_order_with_one_tab_stop(cx: &mut TestAppContext) {
    let evs = vec![timed(1, "First", WORK, 27, 10, 0, 60), timed(2, "Second", WORK, 28, 9, 0, 60), Event::all_day(3, "Third", PERSONAL, d(28), d(28))];
    let (h, shell) = open_week(cx, "week-keys", View::Week, evs);
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("week-scroll").focused(), Some(false), "the grid is the one focus target, not yet focused");
        assert!(w.find_by_label("Second, 09:00 to 10:00, Work").focused().is_none(), "a block is not a tab stop of its own");
        let empty = w.find("week-col-2026-10-30").bounds().center();
        w.pointer_down(empty, MouseButton::Left, cx);
        w.pointer_up(empty, MouseButton::Left, cx);
        for _ in 0..2 {
            w.key_down("right", false, cx);
        }
    })
    .unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).week_roving(), Some(3), "the all-day event comes before the 09:00 one of the same day"));
    cx.update_window(h, |_, w, cx| w.key_down("right", false, cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).week_roving(), Some(2), "right walks on to the last event"));
    let week = cx.update(|cx| shell.read(cx).model.anchor);
    cx.update_window(h, |_, w, cx| w.key_down("right", false, cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.anchor, week + chrono::Duration::days(7), "right at the last event steps to the next week"));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("week-scroll").focused(), Some(true), "one press on the grid focuses it");
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_fifty_events_drag_frame_cost(cx: &mut TestAppContext) {
    let mut evs = Vec::new();
    for i in 0..50u64 {
        let (day, hour) = (26 + (i % 6) as u32, 9 + (i / 6) as u32);
        evs.push(timed(i + 1, &format!("Event {i}"), (i % 3) as u8, day, hour, ((i * 15) % 60) as u32, 45));
    }
    let (h, _) = open_week(cx, "week-perf", View::Week, evs);
    let mut ms: Vec<f64> = Vec::new();
    cx.update_window(h, |_, w, cx| {
        let from = w.find_by_label("Event 7, 10:45 to 11:30, Personal").bounds().center();
        w.pointer_down(from, MouseButton::Left, cx);
        for i in 1..=40 {
            let to = gpui_kit::point(from.x + px(120. * (i % 4) as f32), from.y + px(11. * i as f32 % 160.));
            let t = Instant::now();
            w.pointer_move(to, Some(MouseButton::Left), cx);
            ms.push(t.elapsed().as_secs_f64() * 1000.);
        }
        w.pointer_up(from, MouseButton::Left, cx);
    })
    .unwrap();
    ms.sort_by(f64::total_cmp);
    // Each pointer_move renders two frames (before and after the event), so this is an upper bound for one frame, twice over.
    eprintln!("week 50-event drag, pointer_move incl. 2 frames ({}): median {:.2} ms, max {:.2} ms", if cfg!(debug_assertions) { "debug" } else { "release" }, ms[ms.len() / 2], ms[ms.len() - 1]);
    if !cfg!(debug_assertions) {
        assert!(ms[ms.len() / 2] < 16., "median {} ms", ms[ms.len() / 2]);
    }
}

#[gpui_kit::test]
fn week_scrolls_again_when_the_date_or_the_view_changes(cx: &mut TestAppContext) {
    let now = Rc::new(Cell::new(at(28, 17, 35)));
    let (h, shell) = open_sized(cx, &tmp("week-rescroll"), (1440., 700.), now, |s| s.model.set_view(View::Week));
    let top = |cx: &mut TestAppContext, day: &str| {
        cx.update_window(h, |_, w, cx| {
            (0..3).for_each(|_| w.render_frame(cx));
            f32::from(w.find("week-scroll").bounds().top() - w.find(format!("week-col-{day}")).bounds().top())
        })
        .unwrap()
    };
    let at_now = top(cx, "2026-10-28");
    assert_ne!(at_now, 8. * 44., "today's week opens at the clock (late evening: the end of the day), not at 08:00");
    cx.update_window(h, |_, w, cx| w.click("nav-next", cx)).unwrap();
    assert_eq!(top(cx, "2026-11-02"), 8. * 44., "a week without today opens at 08:00");
    cx.update_window(h, |_, w, cx| w.click("nav-prev", cx)).unwrap();
    assert_eq!(top(cx, "2026-10-28"), at_now, "and back on today's week it returns to the clock");
    let view_h = cx.update_window(h, |_, w, _| f32::from(w.find("week-scroll").bounds().size.height)).unwrap();
    cx.update(|cx| shell.update(cx, |s, cx| {
        s.model.set_now(at(28, 9, 10));
        cx.notify();
    }));
    assert_eq!(top(cx, "2026-10-28"), at_now, "the clock moving does not scroll by itself");
    cx.update_window(h, |_, w, cx| w.click("view-day", cx)).unwrap();
    let want = (9. * 44. + 10. * 44. / 60. - view_h / 3.).max(0.);
    assert!((top(cx, "2026-10-28") - want).abs() < 1., "switching to Day scrolls for its own days: {want}");
}

#[gpui_kit::test]
fn week_a_midnight_crossing_block_is_labelled_for_each_days_piece(cx: &mut TestAppContext) {
    let late = Event::timed(1, "Late", SIDE, at(30, 22, 0), at(31, 1, 0));
    let (h, _) = open_week(cx, "week-late", View::Week, vec![late]);
    cx.update_window(h, |_, w, _| {
        let (fri, sat) = (w.find_by_label("Late, 22:00 to 24:00, Side project").bounds(), w.find_by_label("Late, 00:00 to 01:00, Side project").bounds());
        assert!(fri.left() < sat.left(), "Friday's piece, then Saturday's");
        assert!(w.try_find_by_label("Late, 22:00 Oct 30 to 01:00 Oct 31, Side project").is_none(), "no block carries the whole span");
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_a_stale_roving_event_falls_back_to_the_grid_ring(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-stale-rove", View::Week, vec![timed(1, "Only", WORK, 28, 9, 0, 60)]);
    cx.update_window(h, |_, w, cx| {
        let empty = w.find("week-col-2026-10-30").bounds().center();
        w.pointer_down(empty, MouseButton::Left, cx);
        w.pointer_up(empty, MouseButton::Left, cx);
        w.key_down("right", false, cx);
        w.render_frame(cx);
        assert!(w.try_find("week-ring").is_none(), "the event carries the ring");
    })
    .unwrap();
    cx.update(|cx| {
        shell.update(cx, |s, cx| {
            s.model.toggle_calendar(WORK);
            cx.notify();
        })
    });
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("week-ring").is_some(), "its event is gone: the focused grid still shows where focus is");
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_scroll_on_open_uses_the_viewport_of_the_new_days(cx: &mut TestAppContext) {
    // Next week has two overlapping all-day events: a taller row, so a shorter grid than the week before it.
    let nov_3 = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    let evs = vec![Event::all_day(1, "A", WORK, nov_3, nov_3), Event::all_day(2, "B", PERSONAL, nov_3, nov_3)];
    let (h, _) = open_sized(cx, &tmp("week-viewport"), (1440., 900.), Rc::new(Cell::new(at(28, 12, 0))), |s| {
        s.model.store.events = evs;
        s.model.set_view(View::Week);
    });
    let settle = |cx: &mut TestAppContext| cx.update_window(h, |_, w, cx| (0..4).for_each(|_| w.render_frame(cx))).unwrap();
    settle(cx);
    cx.update_window(h, |_, w, cx| w.click("nav-next", cx)).unwrap();
    settle(cx);
    cx.update_window(h, |_, w, cx| w.click("nav-prev", cx)).unwrap();
    settle(cx);
    cx.update_window(h, |_, w, _| {
        let view = w.find("week-scroll").bounds();
        let scrolled = f32::from(view.top() - w.find("week-col-2026-10-28").bounds().top());
        let want = 12. * 44. - f32::from(view.size.height) / 3.;
        assert!((scrolled - want).abs() < 1., "a third of the way down in the grid as it is now: {scrolled} vs {want}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn day_header_and_all_day_label_follow_mock_v8(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "day-head", View::Day, vec![]);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        // .wh div: 8px padding around the date circle.
        assert_eq!(HEAD_H, DATE_CIRCLE + 16.);
        assert_eq!(w.find("week-head-2026-10-28").bounds().size.height, px(HEAD_H), "header height");
        // .wad aligns "All day" to the row's middle; the row is 30 tall.
        let (row, label) = (w.find("week-all-day").bounds(), w.find("week-all-day-label").bounds());
        assert_eq!(label.center().y, row.top() + px(1. + lane_top(0) + LANE_H / 2.), "label centered on the first lane: border, pad, half a lane");
    })
    .unwrap();
}

#[gpui_kit::test]
fn next_up_banner_is_as_tall_as_the_mocks(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "banner-h", View::Day, vec![timed(1, "Release", WORK, 28, 20, 0, 60)]);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        // .banner: 12 pad, 11px eyebrow at 1.4, 3 + 18px title at 1.4 + 1, 12px line at 1.4 of 13, 10 + 32 button, 12 pad.
        let want = 12. + line_for(BANNER_EYEBROW) + 3. + line_for(BANNER_TITLE) + 1. + line_for(BANNER_META_LINE_OF) + 10. + 32. + 12.;
        let got = f32::from(w.find("banner").bounds().size.height);
        assert!((got - want).abs() < 0.6, "banner height {got}, mock {want}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn all_day_label_stays_on_the_first_lane_with_two_and_three_lanes(cx: &mut TestAppContext) {
    for n in [2u64, 3] {
        let evs = (1..=n).map(|i| Event::all_day(i, "A", WORK, d(28), d(28))).collect();
        let (h, _) = open_week(cx, &format!("lanes-{n}"), View::Day, evs);
        cx.update_window(h, |_, w, cx| {
            w.render_frame(cx);
            let (row, label) = (w.find("week-all-day").bounds(), w.find("week-all-day-label").bounds());
            assert_eq!(label.center().y, row.top() + px(1. + lane_top(0) + LANE_H / 2.), "{n} lanes: label on lane 0");
            assert!(label.top() >= row.top() && label.bottom() <= row.bottom(), "{n} lanes: label inside the row");
        })
        .unwrap();
    }
}
