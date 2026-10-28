//! Headless tests of the week and day views: real presses, drags and releases on the rendered grid.
mod common;

use chrono::NaiveDate;
use common::{at, open_week, open_week_covered, saved};
use gpui_calendar::domain::event::Event;
use gpui_calendar::store::{PERSONAL, Store, WORK};
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Bounds, Entity, MouseButton, Pixels, Point, TestAppContext, Window, point, px};

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

/// The day column's bounds, with the grid's scroll applied.
fn col(w: &Window, day: u32) -> Bounds<Pixels> {
    w.find(format!("week-col-2026-10-{day:02}")).bounds()
}

fn nov_1_col(w: &Window) -> Bounds<Pixels> {
    w.find("week-col-2026-11-01").bounds()
}

/// The pointer position at `minute` (fractional) of `day`, centered across the column. 44 px per hour (guide 5).
fn at_min(w: &Window, day: u32, minute: f32) -> Point<Pixels> {
    let b = col(w, day);
    point(b.center().x, b.top() + px(minute * 44. / 60.))
}

/// Mid-slot minutes, so float error never flips a snap boundary.
const fn hm(h: u32, m: u32) -> f32 {
    (h * 60 + m) as f32 + 0.5
}

fn review() -> Event {
    Event::timed(100, "Review", WORK, at(28, 14, 0), at(28, 15, 0))
}

fn events(cx: &mut TestAppContext, shell: &Entity<Shell>) -> Vec<Event> {
    cx.update(|cx| shell.read(cx).model.store.events.clone())
}

fn event(cx: &mut TestAppContext, shell: &Entity<Shell>, id: u64) -> Event {
    events(cx, shell).into_iter().find(|e| e.id == id).expect("event exists")
}

fn drag(cx: &mut TestAppContext, h: AnyWindowHandle, f: impl Fn(&Window) -> Vec<Point<Pixels>>) {
    cx.update_window(h, |_, w, cx| {
        let path = f(w);
        w.pointer_down(path[0], MouseButton::Left, cx);
        for p in &path[1..] {
            w.pointer_move(*p, Some(MouseButton::Left), cx);
        }
        w.pointer_up(*path.last().unwrap(), MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_dragging_on_the_grid_creates_an_event_snapped_to_15_minutes(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-create", View::Week, vec![]);
    cx.update_window(h, |_, w, cx| {
        w.pointer_down(at_min(w, 28, hm(10, 7)), MouseButton::Left, cx);
        w.pointer_move(at_min(w, 28, hm(11, 20)), Some(MouseButton::Left), cx);
        w.render_frame(cx);
        let ghost = w.find_by_label("New event, 10:00 to 11:30, Work").bounds();
        assert_eq!(ghost.size.height, px(1.5 * 44. - 2.), "the live preview is the snapped block");
        w.pointer_up(at_min(w, 28, hm(11, 20)), MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    let all = events(cx, &shell);
    assert_eq!(all.len(), 1, "exactly one event");
    assert_eq!((all[0].start, all[0].end, all[0].all_day), (at(28, 10, 0), at(28, 11, 30), false));
    assert_eq!(Store::load(&saved("week-create")).unwrap().events.len(), 1, "and it was saved");
}

#[gpui_kit::test]
fn week_a_plain_click_on_the_grid_creates_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-click", View::Week, vec![review()]);
    drag(cx, h, |w| vec![at_min(w, 29, hm(12, 0))]);
    drag(cx, h, |w| vec![at_min(w, 28, hm(14, 20))]);
    assert_eq!(events(cx, &shell), vec![review()]);
    cx.update(|cx| assert_eq!(shell.read(cx).week_roving(), Some(100), "pressing a block makes it the roving event"));
}

#[gpui_kit::test]
fn week_dragging_a_block_to_another_day_and_time_moves_it(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-move", View::Week, vec![review()]);
    drag(cx, h, |w| vec![at_min(w, 28, hm(14, 20)), at_min(w, 30, hm(16, 20))]);
    let e = event(cx, &shell, 100);
    assert_eq!((e.start, e.end), (at(30, 16, 0), at(30, 17, 0)), "two days and two hours later, same length, grab offset kept");
    assert_eq!(events(cx, &shell).len(), 1, "a move never also creates");
}

#[gpui_kit::test]
fn week_dragging_the_bottom_edge_resizes(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-resize", View::Week, vec![review()]);
    drag(cx, h, |w| vec![at_min(w, 28, hm(14, 59) - 0.5 - 0.2), at_min(w, 28, hm(16, 46))]);
    let e = event(cx, &shell, 100);
    assert_eq!((e.start, e.end), (at(28, 14, 0), at(28, 16, 45)), "the end snaps to 16:45, the start stays");
}

#[gpui_kit::test]
fn week_resizing_above_the_start_keeps_15_minutes(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-resize-min", View::Week, vec![review()]);
    drag(cx, h, |w| vec![at_min(w, 28, hm(14, 59) - 0.7), at_min(w, 28, hm(12, 0))]);
    let e = event(cx, &shell, 100);
    assert_eq!((e.start, e.end), (at(28, 14, 0), at(28, 14, 15)));
}

#[gpui_kit::test]
fn week_dragging_a_timed_event_into_the_all_day_row_makes_it_all_day(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-to-all-day", View::Week, vec![review()]);
    drag(cx, h, |w| {
        let row = w.find("week-all-day").bounds();
        vec![at_min(w, 28, hm(14, 20)), point(col(w, 29).center().x, row.top() + px(12.))]
    });
    let e = event(cx, &shell, 100);
    assert!(e.all_day);
    assert_eq!((e.start, e.end), (d(29).and_hms_opt(0, 0, 0).unwrap(), d(30).and_hms_opt(0, 0, 0).unwrap()), "one day later, as the pointer moved");
    cx.update_window(h, |_, w, _| assert!(w.try_find_by_label("Review, Work, all day").is_some(), "now a pill in the all-day row")).unwrap();
}

#[gpui_kit::test]
fn week_an_all_day_event_dropped_on_the_grid_becomes_9_to_10(cx: &mut TestAppContext) {
    let birthday = Event::all_day(7, "Birthday", PERSONAL, d(28), d(28));
    let (h, shell) = open_week(cx, "week-to-grid", View::Week, vec![birthday]);
    drag(cx, h, |w| {
        let from = w.find_by_label("Birthday, Personal, all day").bounds().center();
        vec![from, at_min(w, 30, hm(15, 0))]
    });
    let e = event(cx, &shell, 7);
    assert!(!e.all_day);
    assert_eq!((e.start, e.end), (at(30, 9, 0), at(30, 10, 0)), "09:00 to 10:00 on the drop day, wherever it landed");
}

#[gpui_kit::test]
fn week_a_timed_event_only_converts_when_pressed_on_the_grid(cx: &mut TestAppContext) {
    // Pressing on empty all-day space creates an all-day event across days; it never converts anything.
    let (h, shell) = open_week(cx, "week-all-day-create", View::Week, vec![review()]);
    drag(cx, h, |w| {
        let y = w.find("week-all-day").bounds().top() + px(12.);
        vec![point(col(w, 29).center().x, y), point(col(w, 31).center().x, y)]
    });
    let all = events(cx, &shell);
    assert_eq!(all.len(), 2);
    let made = all.iter().find(|e| e.id != 100).unwrap();
    assert!(made.all_day);
    assert_eq!((made.start.date(), made.end.date()), (d(29), NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()));
    assert_eq!(event(cx, &shell, 100), review());
}

#[gpui_kit::test]
fn week_esc_cancels_a_drag_and_changes_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-esc", View::Week, vec![review()]);
    cx.update_window(h, |_, w, cx| {
        w.pointer_down(at_min(w, 28, hm(14, 20)), MouseButton::Left, cx);
        w.pointer_move(at_min(w, 30, hm(16, 20)), Some(MouseButton::Left), cx);
        w.key_down("escape", false, cx);
        w.pointer_up(at_min(w, 30, hm(16, 20)), MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    assert_eq!(events(cx, &shell), vec![review()]);
}

#[gpui_kit::test]
fn week_a_drag_keeps_tracking_outside_the_grid(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-capture", View::Week, vec![review()]);
    cx.update_window(h, |_, w, cx| {
        w.pointer_down(at_min(w, 28, hm(14, 20)), MouseButton::Left, cx);
        // Past the right edge of the columns, and below the grid: the nearest column and the last minute count.
        let far = point(nov_1_col(w).right() + px(300.), at_min(w, 28, hm(16, 20)).y);
        w.pointer_move(far, Some(MouseButton::Left), cx);
        w.pointer_up(far, MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    let e = event(cx, &shell, 100);
    assert_eq!((e.start.date(), e.start.time()), (NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(), chrono::NaiveTime::from_hms_opt(16, 0, 0).unwrap()));
}

#[gpui_kit::test]
fn day_view_creates_and_moves_in_its_single_column(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "day-drag", View::Day, vec![review()]);
    cx.update_window(h, |_, w, _| {
        assert!(w.try_find("week-col-2026-10-29").is_none(), "one column only");
        assert!(col(w, 28).size.width > px(500.));
    })
    .unwrap();
    drag(cx, h, |w| vec![at_min(w, 28, hm(10, 7)), at_min(w, 28, hm(10, 50))]);
    let made = events(cx, &shell).into_iter().find(|e| e.id != 100).unwrap();
    assert_eq!((made.start, made.end), (at(28, 10, 0), at(28, 11, 0)));
    drag(cx, h, |w| vec![at_min(w, 28, hm(14, 20)), at_min(w, 28, hm(9, 20))]);
    let e = event(cx, &shell, 100);
    assert_eq!((e.start, e.end), (at(28, 9, 0), at(28, 10, 0)));
}

/// Three overlapping events: C (11:30) is the hidden one, so the "+1 more" chip sits at 11:30.
fn crowded() -> Vec<Event> {
    vec![
        Event::timed(1, "A", WORK, at(28, 11, 0), at(28, 12, 0)),
        Event::timed(2, "B", WORK, at(28, 11, 15), at(28, 12, 15)),
        Event::timed(3, "C", WORK, at(28, 11, 30), at(28, 12, 30)),
    ]
}

#[gpui_kit::test]
fn week_pressing_a_block_where_the_chip_once_covered_it_moves_the_block(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-chip-press", View::Week, crowded());
    drag(cx, h, |w| {
        let b = w.find_by_label("B, 11:15 to 12:15, Work").bounds();
        let p = point(b.right() - px(8.), at_min(w, 28, hm(11, 40)).y);
        vec![p, point(p.x, p.y + px(2. * 44.))]
    });
    let e = event(cx, &shell, 2);
    assert_eq!((e.start, e.end), (at(28, 13, 15), at(28, 14, 15)), "B moved two hours; the chip did not take the press");
    assert_eq!(events(cx, &shell).len(), 3);
}

#[gpui_kit::test]
fn week_pressing_the_chip_moves_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-chip-click", View::Week, crowded());
    drag(cx, h, |w| {
        let chip = w.find_by_label("+1 more").bounds();
        vec![chip.center(), point(chip.center().x, chip.center().y + px(88.))]
    });
    assert_eq!(events(cx, &shell), crowded());
}

#[gpui_kit::test]
fn week_a_click_at_the_edge_does_not_scroll_until_it_is_a_drag(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "week-edge", View::Week, vec![]);
    cx.update_window(h, |_, w, cx| {
        let (view, top0) = (w.find("week-scroll").bounds(), col(w, 28).top());
        let p = point(col(w, 28).center().x, view.top() + px(10.));
        w.pointer_down(p, MouseButton::Left, cx);
        for _ in 0..3 {
            w.pointer_move(p, Some(MouseButton::Left), cx);
        }
        assert_eq!(col(w, 28).top(), top0, "still a click: no scroll");
        w.pointer_move(point(p.x, view.top() - px(8.)), Some(MouseButton::Left), cx);
        assert!(col(w, 28).top() > top0, "now a drag at the top edge: the grid scrolls up");
        w.pointer_up(p, MouseButton::Left, cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn week_the_blank_below_a_short_grid_is_not_a_day(cx: &mut TestAppContext) {
    let (h, shell) = common::open_sized(cx, &common::tmp("week-tall"), (1440., 1500.), std::rc::Rc::new(std::cell::Cell::new(at(28, 17, 35))), |s| {
        s.model.store.events = vec![Event::timed(7, "Late", WORK, at(28, 22, 0), at(28, 23, 0))];
        s.model.set_view(View::Week);
    });
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    drag(cx, h, |w| {
        let c = col(w, 29);
        assert!(w.find("week-scroll").bounds().bottom() > c.bottom() + px(60.), "the window is taller than the day");
        vec![point(c.center().x, c.bottom() + px(20.)), point(c.center().x, c.bottom() - px(30.))]
    });
    assert_eq!(events(cx, &shell).len(), 1, "a press below midnight creates nothing");
    drag(cx, h, |w| {
        let c = col(w, 28);
        vec![at_min(w, 28, hm(22, 59) - 0.5 - 0.2 + 1.), point(c.center().x, c.bottom() + px(30.))]
    });
    let e = event(cx, &shell, 7);
    assert_eq!((e.start, e.end), (at(28, 22, 0), at(29, 0, 0)), "dragged past the bottom, the end is midnight");
}

#[gpui_kit::test]
fn week_a_press_on_an_overlay_above_the_grid_starts_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open_week_covered(cx, "week-cover", 300.);
    drag(cx, h, |w| {
        let c = col(w, 31);
        assert!(c.center().x > px(1140.), "Saturday is under the overlay");
        vec![at_min(w, 31, hm(10, 7)), at_min(w, 31, hm(11, 20))]
    });
    assert!(events(cx, &shell).is_empty(), "the press landed on the overlay, not the grid");
    drag(cx, h, |w| vec![at_min(w, 26, hm(10, 7)), at_min(w, 26, hm(11, 20))]);
    assert_eq!(events(cx, &shell).len(), 1, "the same drag on an uncovered column still creates");
}

#[gpui_kit::test]
fn week_the_first_days_bottom_edge_of_a_multi_day_event_moves_it_and_keeps_its_end(cx: &mut TestAppContext) {
    let trip = Event::timed(5, "Trip", WORK, at(29, 22, 0), at(30, 10, 0));
    let (h, shell) = open_week(cx, "week-multi-edge", View::Week, vec![trip]);
    drag(cx, h, |w| vec![at_min(w, 29, hm(23, 57)), at_min(w, 29, hm(22, 57))]);
    let e = event(cx, &shell, 5);
    assert_eq!((e.start, e.end), (at(29, 21, 0), at(30, 9, 0)), "an hour earlier, not an end cut to Thursday night");
}

#[gpui_kit::test]
fn week_a_resize_dragged_over_the_all_day_row_stays_a_resize(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-resize-up", View::Week, vec![review()]);
    drag(cx, h, |w| {
        let row = w.find("week-all-day").bounds();
        vec![at_min(w, 28, hm(14, 59) - 0.5 - 0.2), point(col(w, 28).center().x, row.top() + px(12.))]
    });
    let e = event(cx, &shell, 100);
    assert_eq!((e.all_day, e.start, e.end), (false, at(28, 14, 0), at(28, 14, 15)), "past the top it shrinks to the minimum, it is not cancelled");
}

#[gpui_kit::test]
fn week_a_move_released_over_the_header_does_not_make_the_event_all_day(cx: &mut TestAppContext) {
    let (h, shell) = open_week(cx, "week-move-header", View::Week, vec![review()]);
    drag(cx, h, |w| {
        let row = w.find("week-all-day").bounds();
        vec![at_min(w, 28, hm(14, 20)), point(col(w, 28).center().x, row.top() - px(10.))]
    });
    assert!(!event(cx, &shell, 100).all_day, "only the row itself converts");
}

#[gpui_kit::test]
fn week_a_drag_at_the_edge_keeps_scrolling_while_the_pointer_rests(cx: &mut TestAppContext) {
    let (h, _) = open_week(cx, "week-edge-rest", View::Week, vec![]);
    cx.update_window(h, |_, w, cx| {
        let view = w.find("week-scroll").bounds();
        let from = point(col(w, 28).center().x, view.top() + px(200.));
        w.pointer_down(from, MouseButton::Left, cx);
        w.pointer_move(point(from.x, view.top() + px(6.)), Some(MouseButton::Left), cx);
        let first = col(w, 28).top();
        for _ in 0..4 {
            w.render_frame(cx);
        }
        assert!(col(w, 28).top() > first + px(40.), "four frames at the top edge scrolled further with no pointer move");
        w.pointer_up(from, MouseButton::Left, cx);
        let after = col(w, 28).top();
        for _ in 0..3 {
            w.render_frame(cx);
        }
        assert_eq!(col(w, 28).top(), after, "released: it stops");
    })
    .unwrap();
}
