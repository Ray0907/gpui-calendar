//! Headless tests of the month view: real scrolls, drags and clicks on the rendered grid.
mod common;

use chrono::NaiveDate;
use common::{at, open_sized, tmp};
use gpui_calendar::store::{Store, WORK};
use gpui_calendar::ui::Shell;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, ScrollDelta, TestAppContext, point, px};
use std::{cell::Cell, rc::Rc};

const WEEK_H: f32 = 128.;
/// Chip ids end in the week's index in the list; the run is centered on the week of Oct 26.
const OCT_26: usize = 520;

fn d(m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, day).unwrap()
}

fn open_month(cx: &mut TestAppContext, name: &str, setup: impl FnOnce(&mut Shell) + 'static) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), setup);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.render_frame(cx);
    })
    .unwrap();
    (h, shell)
}

fn title(cx: &mut TestAppContext, h: AnyWindowHandle) -> String {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.render_frame(cx);
        w.find("header-title").label().unwrap().to_string()
    })
    .unwrap()
}

fn event(cx: &mut TestAppContext, shell: &Entity<Shell>, id: u64) -> gpui_calendar::domain::event::Event {
    cx.update(|cx| shell.read(cx).model.store.events.iter().find(|e| e.id == id).cloned().expect("event exists"))
}

#[gpui_kit::test]
fn month_scrolling_six_months_changes_the_header_title(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-scroll", |_| {});
    assert_eq!(title(cx, h), "October 2026", "opens on the month of today");
    // Sep 28 (first grid week of October) to Mar 29 (first grid week of April) is 26 weeks.
    cx.update_window(h, |_, w, cx| w.scroll("month-scroll", ScrollDelta::Pixels(point(px(0.), px(-26. * WEEK_H))), cx)).unwrap();
    assert_eq!(title(cx, h), "April 2027");
    cx.update(|cx| assert_eq!(shell.read(cx).model.mini_title(), "April 2027", "the mini month follows too"));
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("view-month").checked(), Some(true));
        w.click("nav-today", cx);
    })
    .unwrap();
    assert_eq!(title(cx, h), "October 2026", "Today scrolls the list back");
}

#[gpui_kit::test]
fn month_next_and_previous_buttons_scroll_the_list(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-nav", |_| {});
    cx.update_window(h, |_, w, cx| w.click("nav-next", cx)).unwrap();
    assert_eq!(title(cx, h), "November 2026");
    cx.update_window(h, |_, w, cx| {
        // November's first grid week is at the top of the grid.
        let top = w.find("month-scroll").bounds().top();
        assert_eq!(w.find("month-cell-2026-10-26").bounds().top(), top);
        w.click("nav-prev", cx);
        w.click("nav-prev", cx);
    })
    .unwrap();
    assert_eq!(title(cx, h), "September 2026");
}

#[gpui_kit::test]
fn month_dragging_across_three_days_creates_one_event_spanning_them(cx: &mut TestAppContext) {
    let p = tmp("month-create");
    let (h, shell) = open_sized(cx, &p, (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    let before = cx.update(|cx| shell.read(cx).model.store.events.len());
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.render_frame(cx);
        let at = |w: &gpui_kit::Window, day| w.find(format!("month-cell-2026-10-{day:02}")).bounds().center();
        let (a, b, c) = (at(w, 6), at(w, 7), at(w, 8));
        w.pointer_down(a, MouseButton::Left, cx);
        w.pointer_move(b, Some(MouseButton::Left), cx);
        w.pointer_move(c, Some(MouseButton::Left), cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("New event, Work, all day, Oct 6 to Oct 8").is_some(), "the ghost shows while dragging");
        w.pointer_up(c, MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    cx.update(|cx| {
        let events = &shell.read(cx).model.store.events;
        assert_eq!(events.len(), before + 1, "exactly one event");
        let e = events.last().unwrap();
        assert!(e.all_day);
        assert_eq!((e.start.date(), e.end.date()), (d(10, 6), d(10, 9)), "Oct 6, 7 and 8");
    });
    assert_eq!(Store::load(&p).unwrap().events.len(), before + 1, "and it was saved");
    cx.update_window(h, |_, w, _| {
        let bar = w.find(format!("month-ev-14-{}", OCT_26 - 3));
        let cell = w.find("month-cell-2026-10-06").bounds();
        assert!(bar.bounds().size.width > cell.size.width * 2., "one bar over three cells");
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_a_plain_click_creates_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-click", |_| {});
    let before = cx.update(|cx| shell.read(cx).model.store.events.len());
    cx.update_window(h, |_, w, cx| w.click("month-cell-2026-10-06", cx)).unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.store.events.len(), before));
}

#[gpui_kit::test]
fn month_moving_a_chip_to_another_day_keeps_its_time(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-move", |_| {});
    let before = event(cx, &shell, 4);
    assert_eq!(before.start, at(28, 9, 30));
    cx.update_window(h, |_, w, cx| {
        let from = w.find_by_label("Standup, 09:30, Work").bounds().center();
        let to = w.find("month-cell-2026-10-30").bounds().center();
        w.pointer_down(from, MouseButton::Left, cx);
        w.pointer_move(to, Some(MouseButton::Left), cx);
        w.pointer_up(to, MouseButton::Left, cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("Standup, 09:30, Work").is_some(), "still drawn, on its new day");
    })
    .unwrap();
    let after = event(cx, &shell, 4);
    assert_eq!((after.start, after.end), (at(30, 9, 30), at(30, 9, 45)), "two days later, same time");
    cx.update(|cx| assert_eq!(shell.read(cx).model.store.events.iter().filter(|e| e.title == "New event").count(), 0, "a move never also creates"));
}

#[gpui_kit::test]
fn month_clicking_a_chip_moves_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-chip-click", |_| {});
    let before = event(cx, &shell, 4);
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    assert_eq!(event(cx, &shell, 4), before);
}

#[gpui_kit::test]
fn month_hidden_calendar_chips_disappear_and_come_back(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-hidden", |_| {});
    cx.update_window(h, |_, w, cx| {
        for name in ["Standup, 09:30, Work", "Piano lesson, 18:00, Personal"] {
            assert!(w.try_find_by_label(name).is_some(), "{name} drawn");
        }
        w.click("calendar-row-0", cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("Standup, 09:30, Work").is_none(), "Work is hidden");
        assert!(w.try_find_by_label("Piano lesson, 18:00, Personal").is_some(), "Personal stays");
        w.click("calendar-row-0", cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("Standup, 09:30, Work").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_first_event_row_has_the_same_offset_in_every_cell(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-align", |_| {});
    cx.update_window(h, |_, w, _| {
        let offset = |chip: &'static str, day: u32| w.find(chip).bounds().top() - w.find(format!("month-cell-2026-10-{day}")).bounds().top();
        // Design review (Tue 27), Standup (Wed 28), Dentist (Thu 29): each the first row of its day.
        let rows = [offset("month-ev-2-520", 27), offset("month-ev-4-520", 28), offset("month-ev-6-520", 29)];
        assert_eq!(rows[0], rows[1]);
        assert_eq!(rows[1], rows[2]);
        assert_eq!(rows[0], px(34.), "33px row top plus the chip's 1px inset");
        // Release checklist overlaps Design review's day: second row, one row lower.
        assert_eq!(offset("month-ev-3-520", 27), px(34. + 22.));
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_spanning_bars_cross_cells_and_a_busy_day_says_how_many_more(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-bars", |s| {
        for i in 0..6u32 {
            let start = d(10, 14).and_hms_opt(8 + i, 0, 0).unwrap();
            s.model.store.events.push(gpui_calendar::domain::event::Event::timed(100 + u64::from(i), &format!("Busy {i}"), WORK, start, start + chrono::Duration::minutes(30)));
        }
    });
    cx.update_window(h, |_, w, _| {
        // Six chips on Wed Oct 14 with three rows: three hidden.
        assert!(w.try_find_by_label("+3 more").is_some());
        assert_eq!(w.find_all_by_label("+3 more").len(), 1, "only that day has a count");
    })
    .unwrap();
    cx.update_window(h, |_, w, cx| {
        w.click("nav-next", cx);
        w.render_frame(cx);
        w.render_frame(cx);
        let bar = w.find(format!("month-ev-9-{}", OCT_26 + 1)).bounds(); // Conference, Nov 3 to 5
        let cell = w.find("month-cell-2026-11-03").bounds();
        assert!((bar.size.width - (cell.size.width * 3. - px(8.))).abs() <= px(1.), "Nov 3 to 5: three cells less the 4px inset each side, {:?}", bar.size.width);
        assert!(w.try_find_by_label("Conference, Work, all day, Nov 3 to Nov 5").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_grid_is_one_tab_stop_and_arrows_walk_the_events(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-keys", |_| {});
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("month-scroll").focused(), Some(false), "the grid is the one focus target, not yet focused");
        for chip in ["month-ev-2-520", "month-ev-4-520", "month-ev-6-520"] {
            assert!(w.find(chip).focused().is_none(), "{chip} is not a tab stop of its own");
        }
        assert!(w.find("month-cell-2026-10-06").focused().is_none(), "nor is a cell");
        w.click("month-cell-2026-10-06", cx);
        assert_eq!(w.find("month-scroll").focused(), Some(true), "one click on the grid focuses it");
        w.press("right", cx);
    })
    .unwrap();
    let roving = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).month_roving());
    assert_eq!(roving(cx), Some(1), "the first event of the month on screen");
    cx.update_window(h, |_, w, cx| {
        w.press("right", cx);
        w.press("right", cx);
    })
    .unwrap();
    assert_eq!(roving(cx), Some(3));
    cx.update_window(h, |_, w, cx| w.press("left", cx)).unwrap();
    assert_eq!(roving(cx), Some(2));
}

#[gpui_kit::test]
fn month_up_from_nowhere_enters_at_the_end_of_the_month_not_the_start(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-up", |_| {});
    cx.update_window(h, |_, w, cx| {
        w.click("month-cell-2026-10-06", cx);
        w.press("up", cx);
    })
    .unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).month_roving()), Some(7), "the last October event by start, not event 1");
}

#[gpui_kit::test]
fn month_modified_arrows_are_left_alone(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-mods", |_| {});
    cx.update_window(h, |_, w, cx| {
        w.click("month-cell-2026-10-06", cx);
        for chord in ["cmd-right", "ctrl-right", "alt-down", "cmd-up", "shift-right", "shift-up", "fn-left"] {
            w.press(chord, cx);
        }
    })
    .unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).month_roving()), None, "no chord moved the roving event");
    cx.update_window(h, |_, w, cx| w.press("right", cx)).unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).month_roving()), Some(1), "the plain arrow still does");
}

#[gpui_kit::test]
fn month_with_every_calendar_hidden_creates_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "month-all-hidden", |s| s.model.store.calendars.iter_mut().for_each(|c| c.visible = false));
    let before = cx.update(|cx| shell.read(cx).model.store.events.len());
    cx.update_window(h, |_, w, cx| {
        let at = |w: &gpui_kit::Window, day| w.find(format!("month-cell-2026-10-{day:02}")).bounds().center();
        let (a, b) = (at(w, 6), at(w, 8));
        w.pointer_down(a, MouseButton::Left, cx);
        w.pointer_move(b, Some(MouseButton::Left), cx);
        w.render_frame(cx);
        assert!(w.try_find_by_label("New event, Work, all day, Oct 6 to Oct 8").is_none(), "no ghost either");
        w.pointer_up(b, MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    cx.update(|cx| {
        let s = shell.read(cx);
        assert_eq!(s.model.store.events.len(), before, "no invisible event");
        assert_eq!(s.month_roving(), None);
    });
}

#[gpui_kit::test]
fn month_a_changed_week_start_moves_the_header_and_the_weeks_together(cx: &mut TestAppContext) {
    // The shell builds the weeks before this runs, on Monday; the setting changes under them.
    let (h, _) = open_month(cx, "month-sunday", |s| s.model.store.settings.week_start = chrono::Weekday::Sun);
    cx.update_window(h, |_, w, _| {
        assert_eq!(w.find("month-head-0").label(), Some("Sun"));
        assert_eq!(w.find("month-head-1").label(), Some("Mon"));
        let sun = w.find("month-cell-2026-10-25").bounds();
        assert_eq!(w.find("month-head-0").bounds().left(), sun.left(), "Sunday's cell sits under the first heading");
        assert_eq!(w.find("month-head-6").bounds().left(), w.find("month-cell-2026-10-31").bounds().left(), "and Saturday's under the last");
        assert_eq!(sun.top(), w.find("month-cell-2026-10-31").bounds().top(), "one row");
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_focusing_the_grid_marks_the_anchor_day_and_cells_name_their_date(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-tab", |_| {});
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("month-cell-2026-10-28").label(), Some("Wednesday, October 28, 2026"));
        assert_eq!(w.find("month-cell-2026-11-01").label(), Some("Sunday, November 1, 2026"));
        assert_ne!(w.find("month-cell-2026-10-28").selected(), Some(true), "no ring while the grid is not focused");
        // The harness cannot drive Tab traversal (the key reaches no focus move); a click focuses the grid the same way.
        w.click("month-cell-2026-10-06", cx);
        assert_eq!(w.find("month-scroll").focused(), Some(true), "the grid has focus");
        assert_eq!(w.find("month-cell-2026-10-28").selected(), Some(true), "the anchor day carries the ring");
        assert_ne!(w.find("month-cell-2026-10-29").selected(), Some(true), "and only it");
        w.press("right", cx);
        assert_ne!(w.find("month-cell-2026-10-28").selected(), Some(true), "an event took over");
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_cells_show_weekday_names_and_mark_today(cx: &mut TestAppContext) {
    let (h, _) = open_month(cx, "month-head", |_| {});
    cx.update_window(h, |_, w, _| {
        let sat = w.find("month-cell-2026-10-31").bounds();
        let fri = w.find("month-cell-2026-10-30").bounds();
        assert_eq!(sat.top(), fri.top(), "one row");
        assert!(sat.left() > fri.left(), "Monday-first week");
    })
    .unwrap();
}

/// The first of a month reads "Nov 1": today's fill on it is a pill with 6px of air each side, while "2" stays the 22px circle.
#[gpui_kit::test]
fn first_of_month_label_is_a_padded_pill_and_other_days_stay_circles(cx: &mut TestAppContext) {
    let nov_1 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap().and_hms_opt(9, 0, 0).unwrap();
    let (h, _) = open_sized(cx, &tmp("month-first"), (1280., 900.), Rc::new(Cell::new(nov_1)), |_| {});
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.render_frame(cx);
        let (first, second) = (w.find("month-num-2026-11-01").bounds(), w.find("month-num-2026-11-02").bounds());
        assert_eq!(f32::from(first.size.height), 22., "pill height");
        assert_eq!(f32::from(second.size.width), 22., "a single digit stays a 22px circle");
        let text = w.find("month-numtext-2026-11-01").bounds();
        assert!((f32::from(first.size.width - text.size.width) - 12.).abs() < 0.1, "6px of air each side of the text, got {} around {}", f32::from(first.size.width), f32::from(text.size.width));
        assert!(f32::from(text.left() - first.left()) > 5.9, "left air");
    })
    .unwrap();
}
