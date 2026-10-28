//! Headless tests of the Year view: clicks, steps, dots, arrow keys and the single Tab stop.
mod common;

use common::year::*;
use common::{at, open_sized, tmp};
use gpui_calendar::domain::year::Cursor;
use gpui_calendar::store::{PERSONAL, SIDE, WORK};
use gpui_kit::Role;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, MouseButton, TestAppContext};
use std::{cell::Cell, rc::Rc};

fn title(cx: &mut TestAppContext, h: AnyWindowHandle) -> String {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.find("header-title").label().unwrap().to_string()
    })
    .unwrap()
}

#[gpui_kit::test]
fn year_shows_twelve_months_and_the_year_in_the_header(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-open", 1440.);
    assert_eq!(title(cx, h), "2026");
    cx.update_window(h, |_, w, _| {
        for m in 1..=12 {
            let name = d(2026, m, 1).format("%B 2026").to_string();
            assert!(w.try_find(format!("year-month-2026-{m:02}")).is_some(), "{name} is there");
            assert_eq!(w.find(format!("year-month-2026-{m:02}")).label().unwrap(), format!("{name}, open in Month view"));
        }
        assert_eq!(w.find("view-year").checked(), Some(true));
        assert!(w.try_find("year-day-2026-10-28").is_some() && w.try_find("year-day-2026-01-01").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn clicking_a_day_opens_the_day_view_on_that_date(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-day", 1440.);
    cx.update_window(h, |_, w, cx| w.click("year-day-2026-10-07", cx)).unwrap();
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!((m.view, m.anchor), (View::Day, d(2026, 10, 7)));
    });
    assert_eq!(title(cx, h), "Wed 7 Oct 2026");
}

#[gpui_kit::test]
fn clicking_a_month_title_opens_the_month_view_on_that_month(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-month", 1440.);
    cx.update_window(h, |_, w, cx| w.click("year-month-2026-11", cx)).unwrap();
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!((m.view, m.anchor), (View::Month, d(2026, 11, 1)));
    });
    assert_eq!(title(cx, h), "November 2026");
}

#[gpui_kit::test]
fn previous_and_next_step_one_year(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-step", 1440.);
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("nav-next").label().unwrap(), "Next year");
        assert_eq!(w.find("nav-prev").label().unwrap(), "Previous year");
        w.click("nav-next", cx);
    })
    .unwrap();
    assert_eq!(title(cx, h), "2027");
    cx.update_window(h, |_, w, cx| {
        assert!(w.try_find("year-day-2027-01-01").is_some() && w.try_find("year-day-2026-01-01").is_none());
        w.click("nav-prev", cx);
        w.click("nav-prev", cx);
    })
    .unwrap();
    assert_eq!(title(cx, h), "2025");
    cx.update_window(h, |_, w, cx| w.click("nav-today", cx)).unwrap();
    assert_eq!(title(cx, h), "2026");
}

#[gpui_kit::test]
fn dots_are_one_per_calendar_and_hidden_calendars_lose_theirs(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-dots", 1440.);
    cx.update_window(h, |_, w, _| {
        let busy = "Wednesday, October 14, 2026, 3 events";
        assert!(w.try_find_by_label(busy).is_some());
        for cal in [WORK, PERSONAL, SIDE] {
            assert!(w.try_find(format!("year-dot-2026-10-14-{cal}")).is_some(), "calendar {cal} has a dot on the 14th");
        }
        assert!(w.try_find_by_label("Tuesday, October 27, 2026, 1 event").is_some());
        assert!(w.try_find_by_label("Thursday, October 29, 2026, no events").is_some());
        assert!(w.try_find("year-dot-2026-10-29-0").is_none());
        assert_eq!(w.find_by_label("Wednesday, October 28, 2026, no events").description(), Some("current date"), "today is named");
    })
    .unwrap();
    cx.update(|cx| shell.update(cx, |s, cx| {
        s.model.toggle_calendar(PERSONAL);
        s.model.toggle_calendar(WORK);
        s.sync(cx);
    }));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("year-dot-2026-10-14-0").is_none() && w.try_find("year-dot-2026-10-14-1").is_none(), "hidden calendars lose their dots");
        assert!(w.try_find("year-dot-2026-10-14-2").is_some(), "the visible one keeps its dot");
        assert!(w.try_find_by_label("Wednesday, October 14, 2026, 1 event").is_some(), "and the count drops");
        assert!(w.try_find("year-dot-2026-10-27-0").is_none());
        assert!(w.try_find_by_label("Tuesday, October 27, 2026, no events").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn arrows_move_one_roving_day_across_days_and_months(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-keys", 1440.);
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("year-scroll").focused(), Some(false), "the grid is the one focus target, not yet focused");
        assert!(w.find("year-day-2026-10-28").focused().is_none(), "a day is not a tab stop of its own");
        assert!(w.find("year-month-2026-10").focused().is_none(), "nor is a month title");
        // A press on the grid's padding focuses it without choosing a day.
        let pad = w.find("year-scroll").bounds().origin + gpui_kit::point(gpui_kit::px(4.), gpui_kit::px(4.));
        w.pointer_down(pad, MouseButton::Left, cx);
        w.pointer_up(pad, MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("year-scroll").focused(), Some(true));
        assert!(w.try_find(ring_of("year-day-2026-10-28")).is_some(), "the roving day starts on the anchor");
        w.key_down("right", false, cx);
        w.key_down("right", false, cx);
        w.key_down("right", false, cx);
        w.key_down("right", false, cx); // Nov 1, over the month edge
        w.render_frame(cx);
        assert!(w.try_find(ring_of("year-day-2026-11-01")).is_some());
        assert!(!w.try_find(ring_of("year-day-2026-10-28")).is_some(), "only one day is the cursor");
        w.key_down("down", false, cx);
        w.key_down("left", false, cx);
        w.render_frame(cx);
        assert!(w.try_find(ring_of("year-day-2026-11-07")).is_some());
        w.key_down("up", false, cx);
        w.render_frame(cx);
        assert!(w.try_find(ring_of("year-day-2026-10-31")).is_some(), "up goes a week back, into October");
    })
    .unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.year_cursor, Some(Cursor::Day(d(2026, 10, 31)))));
    cx.update_window(h, |_, w, cx| {
        w.key_down("enter", false, cx);
    })
    .unwrap();
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!((m.view, m.anchor), (View::Day, d(2026, 10, 31)), "Enter opens the roving day");
    });
}

#[gpui_kit::test]
fn the_year_view_is_two_columns_when_narrow(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-narrow", 700.);
    cx.update_window(h, |_, w, _| {
        let (jan, feb, mar) = (w.find("year-month-2026-01").bounds(), w.find("year-month-2026-02").bounds(), w.find("year-month-2026-03").bounds());
        assert_eq!(jan.top(), feb.top(), "January and February share a row");
        assert!(mar.top() > jan.top(), "March starts the next row");
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_today_circle_and_its_dots_have_room_at_1440_and_at_the_minimum_window(cx: &mut TestAppContext) {
    for (name, (w, hgt)) in [("year-geo-big", (1440., 900.)), ("year-geo-min", (960., 600.))] {
        let (h, _) = open_sized(cx, &tmp(name), (w, hgt), Rc::new(Cell::new(at(28, 17, 35))), |s| {
            s.model.store.events = vec![timed(1, "Standup", WORK, 28, 9), timed(2, "Dentist", PERSONAL, 28, 12)];
            s.model.set_view(View::Year);
        });
        cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
        cx.update_window(h, |_, w, _| {
            let num = w.find("year-num-2026-10-28").bounds();
            assert!(num.size.height >= gpui_kit::px(22.) && num.size.width >= gpui_kit::px(22.), "{name}: the date box is not squashed, got {:?}", num.size);
            assert_eq!(num.size.width, num.size.height, "{name}: the today circle is a circle");
            for cal in [WORK, PERSONAL] {
                let dot = w.find(format!("year-dot-2026-10-28-{cal}")).bounds();
                assert!(dot.top() >= num.bottom(), "{name}: dot {cal} sits under the circle, not on it");
            }
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn up_from_a_first_row_day_reaches_the_month_title_and_enter_opens_the_month(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-title-key", 1440.);
    focus_grid(cx, h);
    // Anchor Oct 28: three weeks up is Oct 7; Oct 4 is the end of the first row, and up from there is the title.
    keys(cx, h, &["up", "up", "up", "left", "left", "left", "up"]);
    assert!(ringed(cx, h, "year-month-2026-10"), "the title is a roving stop");
    assert!(!ringed(cx, h, "year-day-2026-10-07"));
    keys(cx, h, &["right"]);
    assert!(ringed(cx, h, "year-month-2026-11"), "sideways on titles goes by month");
    keys(cx, h, &["left", "enter"]);
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!((m.view, m.anchor), (View::Month, d(2026, 10, 1)));
    });
}

#[gpui_kit::test]
fn the_roving_stop_resets_on_today_prev_next_and_view_changes(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-reset", 1440.);
    let cursor = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).model.year_cursor);
    focus_grid(cx, h);
    keys(cx, h, &["right", "right", "right", "right", "right", "right"]);
    assert_eq!(cursor(cx), Some(Cursor::Day(d(2026, 11, 3))));
    cx.update_window(h, |_, w, cx| w.click("nav-today", cx)).unwrap();
    assert_eq!(cursor(cx), None, "Today clears the roving day");
    focus_grid(cx, h);
    assert!(ringed(cx, h, "year-day-2026-10-28"), "the cursor is back on today, not Nov 3");
    keys(cx, h, &["pagedown"]);
    assert_eq!(cursor(cx), Some(Cursor::Day(d(2026, 11, 28))));
    cx.update_window(h, |_, w, cx| {
        w.click("nav-next", cx);
        w.click("nav-prev", cx);
    })
    .unwrap();
    assert_eq!(cursor(cx), None, "Next then Prev does not bring the old day back");
    focus_grid(cx, h);
    keys(cx, h, &["right"]);
    assert!(cursor(cx).is_some());
    cx.update_window(h, |_, w, cx| w.click("view-month", cx)).unwrap();
    assert_eq!(cursor(cx), None, "a view change clears it");
}

#[gpui_kit::test]
fn space_acts_like_enter_and_home_end_page_keys_move_the_cursor(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-more-keys", 1440.);
    focus_grid(cx, h);
    let cursor = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).model.year_cursor);
    keys(cx, h, &["home"]);
    assert_eq!(cursor(cx), Some(Cursor::Day(d(2026, 10, 26))), "Home: Monday of the week");
    keys(cx, h, &["end"]);
    assert_eq!(cursor(cx), Some(Cursor::Day(d(2026, 10, 31))), "End stays on the last October day, not the Nov 1 filler");
    keys(cx, h, &["pageup"]);
    assert_eq!(cursor(cx), Some(Cursor::Day(d(2026, 9, 30))), "a month back from the 31st: the last day of September");
    keys(cx, h, &["space"]);
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!((m.view, m.anchor), (View::Day, d(2026, 9, 30)), "Space opens the roving day");
    });
}

#[gpui_kit::test]
fn out_of_month_filler_days_are_not_a_second_button(cx: &mut TestAppContext) {
    let (h, shell) = open_year(cx, "year-filler", 1440.);
    // The snapshot keys elements by id, so a duplicate button would hide in it: count what the frame built.
    cx.update(|cx| assert_eq!(shell.read(cx).year_day_buttons(), 365, "a button per day of the year, none for the fillers"));
    cx.update_window(h, |_, w, _| {
        // October 2026 starts with Sep 28 to 30 and ends with Nov 1 as filler.
        assert!(w.try_find("year-day-2026-09-28").is_some() && w.try_find("year-day-2026-11-01").is_some(), "their own months have the buttons");
        let named = w.find_all_by_label("Monday, September 28, 2026, no events");
        assert_eq!(named.len(), 1, "one button for Sep 28, not one per mini month it shows in");
        let buttons = w.find_all_by_role(Role::Button).iter().filter(|e| e.label().is_some_and(|l| l.contains("2026, ") && !l.ends_with("Month view"))).count();
        assert_eq!(buttons, 365, "exactly one named button per day of the year");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_fourth_calendar_shows_a_plus_marker_and_the_unthemed_one_is_counted(cx: &mut TestAppContext) {
    let (h, _) = open_sized(cx, &tmp("year-plus"), (1440., 900.), Rc::new(Cell::new(at(28, 17, 35))), |s| {
        s.model.store.events = vec![timed(1, "A", WORK, 14, 9), timed(2, "B", PERSONAL, 14, 10), timed(3, "C", SIDE, 14, 11), timed(4, "D", 3, 14, 12)];
        s.model.set_view(View::Year);
    });
    cx.update_window(h, |_, w, cx| {
        (0..3).for_each(|_| w.render_frame(cx));
        assert!(w.try_find_by_label("Wednesday, October 14, 2026, 4 events").is_some());
        let dots = [WORK, PERSONAL, SIDE, 3].iter().filter(|c| w.try_find(format!("year-dot-2026-10-14-{c}")).is_some()).count();
        let more = w.try_find("year-more-2026-10-14");
        assert!(more.is_some(), "four calendars: a marker, so dots and the event count agree");
        assert_eq!(dots + 1, 4, "three dots plus the marker account for four calendars");
        assert!(w.try_find("year-more-2026-10-27").is_none(), "no marker on a day within the dots");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_scrolled_away_cursor_gets_a_hint_and_arrowing_back_hides_it(cx: &mut TestAppContext) {
    let (h, _) = open_sized(cx, &tmp("year-hint"), (960., 600.), Rc::new(Cell::new(at(28, 17, 35))), |s| s.model.set_view(View::Year));
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    focus_grid(cx, h);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("year-focus-hint").is_none(), "the view opens on October, where the cursor is");
        w.scroll("year-scroll", gpui_kit::ScrollDelta::Pixels(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(2000.))), cx);
        (0..2).for_each(|_| w.render_frame(cx));
        assert!(w.try_find("year-focus-hint").is_some(), "scrolled to January, the cursor is off screen: the grid says where it is");
    })
    .unwrap();
    keys(cx, h, &["right"]);
    cx.update_window(h, |_, w, cx| {
        (0..2).for_each(|_| w.render_frame(cx));
        assert!(w.try_find("year-focus-hint").is_none(), "a key step scrolls the cursor into view");
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_view_opens_with_the_anchor_month_on_screen_and_today_brings_it_back(cx: &mut TestAppContext) {
    let (h, _) = open_year(cx, "year-placed", 1440.);
    let on_screen = |cx: &mut TestAppContext| {
        cx.update_window(h, |_, w, cx| {
            (0..2).for_each(|_| w.render_frame(cx));
            let (view, num) = (w.find("year-scroll").bounds(), w.find("year-num-2026-10-28").bounds());
            num.top() >= view.top() && num.bottom() <= view.bottom()
        })
        .unwrap()
    };
    assert!(on_screen(cx), "October, the anchor's month, is scrolled to at 1440x900");
    focus_grid(cx, h);
    keys(cx, h, &["pageup"; 9]);
    assert!(!on_screen(cx), "the cursor went to January, October is away");
    cx.update_window(h, |_, w, cx| w.click("nav-today", cx)).unwrap();
    assert!(on_screen(cx), "Today scrolls October back in");
}
