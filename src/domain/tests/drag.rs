use chrono::{NaiveDate, NaiveDateTime};

use crate::domain::drag::*;
use crate::domain::event::Event;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
fn dt(y: i32, m: u32, day: u32, h: u32, mi: u32) -> NaiveDateTime {
    d(y, m, day).and_hms_opt(h, mi, 0).unwrap()
}
fn min(h: i32, m: i32) -> i32 {
    h * 60 + m
}
fn timed() -> Event {
    Event::timed(1, "t", 0, dt(2026, 10, 28, 10, 0), dt(2026, 10, 28, 11, 0))
}

// ---- create on grid

#[test]
fn create_grid_drag_down_snaps_outward() {
    let mut g = Drag::create(Pos::Grid(d(2026, 10, 28), min(9, 7)));
    g.update(Pos::Grid(d(2026, 10, 28), min(10, 20)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 30), false));
    assert!(g.moved());
}

#[test]
fn create_grid_drag_up_is_symmetric() {
    let mut g = Drag::create(Pos::Grid(d(2026, 10, 28), min(10, 20)));
    g.update(Pos::Grid(d(2026, 10, 28), min(9, 7)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 30)));
}

#[test]
fn create_grid_click_has_min_15_minutes_and_not_moved() {
    let g = Drag::create(Pos::Grid(d(2026, 10, 28), min(9, 0)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 9, 15)));
    assert!(!g.moved());
}

#[test]
fn create_grid_stays_in_anchor_day_and_clamps_midnight() {
    let mut g = Drag::create(Pos::Grid(d(2026, 10, 28), min(23, 0)));
    g.update(Pos::Grid(d(2026, 10, 29), min(5, 0)));
    let p = g.preview();
    assert_eq!(p.start, dt(2026, 10, 28, 23, 0));
    assert_eq!(p.end, dt(2026, 10, 29, 0, 0));
}

// ---- create all-day / month

#[test]
fn create_month_across_days_is_all_day_inclusive() {
    let mut g = Drag::create(Pos::Month(d(2026, 10, 30)));
    g.update(Pos::Month(d(2026, 11, 2)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 30, 0, 0), dt(2026, 11, 3, 0, 0), true));
    g.update(Pos::Month(d(2026, 10, 28))); // backwards
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 0, 0), dt(2026, 10, 31, 0, 0)));
}

#[test]
fn create_in_all_day_row_matches_month() {
    let mut g = Drag::create(Pos::AllDay(d(2026, 10, 28)));
    g.update(Pos::AllDay(d(2026, 10, 29)));
    let p = g.preview();
    assert!(p.all_day);
    assert_eq!(p.end, dt(2026, 10, 30, 0, 0));
}

#[test]
fn create_ignores_updates_from_other_surface() {
    let mut g = Drag::create(Pos::Grid(d(2026, 10, 28), min(9, 0)));
    let before = g.preview();
    g.update(Pos::Month(d(2026, 11, 1)));
    assert_eq!(g.preview(), before);
}

// ---- move on grid

#[test]
fn move_grid_keeps_duration_and_grab_offset() {
    let e = timed(); // 10:00-11:00
    let mut g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(10, 20))); // grabbed 20 min in
    g.update(Pos::Grid(d(2026, 10, 29), min(14, 20)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 29, 14, 0), dt(2026, 10, 29, 15, 0), false));
    g.update(Pos::Grid(d(2026, 10, 29), min(14, 31))); // snaps to nearest 15
    assert_eq!(g.preview().start, dt(2026, 10, 29, 14, 15));
}

#[test]
fn move_grid_can_cross_midnight() {
    let e = timed();
    let mut g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(10, 0)));
    g.update(Pos::Grid(d(2026, 10, 28), min(23, 30)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 23, 30), dt(2026, 10, 29, 0, 30)));
}

#[test]
fn move_without_motion_is_identity_and_not_moved() {
    let e = timed();
    let g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(10, 20)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (e.start, e.end));
    assert!(!g.moved());
    assert_eq!(g.apply(&e), e);
}

// ---- conversions

#[test]
fn timed_into_all_day_row_becomes_all_day_on_drop_day() {
    let e = timed();
    let mut g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(10, 20)));
    g.update(Pos::AllDay(d(2026, 10, 30)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 30, 0, 0), dt(2026, 10, 31, 0, 0), true));
    let out = g.apply(&e);
    assert!(out.all_day);
    assert_eq!(out.id, 1);
    assert_eq!(out.title, "t");
}

#[test]
fn all_day_onto_grid_becomes_nine_to_ten() {
    let e = Event::all_day(2, "a", 1, d(2026, 10, 28), d(2026, 10, 28));
    let mut g = Drag::move_event(&e, Pos::AllDay(d(2026, 10, 28)));
    g.update(Pos::Grid(d(2026, 10, 30), min(15, 40)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 30, 9, 0), dt(2026, 10, 30, 10, 0), false));
}

#[test]
fn conversion_is_reversible_within_one_drag() {
    let e = timed();
    let mut g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(10, 0)));
    g.update(Pos::AllDay(d(2026, 10, 28)));
    assert!(g.preview().all_day);
    g.update(Pos::Grid(d(2026, 10, 28), min(10, 0)));
    let p = g.preview();
    assert!(!p.all_day);
    assert_eq!((p.start, p.end), (e.start, e.end));
}

#[test]
fn multi_day_all_day_moves_keep_span_in_row() {
    let e = Event::all_day(3, "trip", 0, d(2026, 10, 27), d(2026, 10, 29)); // 3 days
    let mut g = Drag::move_event(&e, Pos::AllDay(d(2026, 10, 28))); // grabbed middle day
    g.update(Pos::AllDay(d(2026, 11, 2)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 11, 1, 0, 0), dt(2026, 11, 4, 0, 0)));
}

// ---- month move

#[test]
fn month_chip_move_keeps_time() {
    let e = timed();
    let mut g = Drag::move_event(&e, Pos::Month(d(2026, 10, 28)));
    g.update(Pos::Month(d(2026, 11, 3)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 11, 3, 10, 0), dt(2026, 11, 3, 11, 0), false));
}

#[test]
fn month_chip_move_all_day_multi_day_keeps_span() {
    let e = Event::all_day(3, "trip", 0, d(2026, 10, 27), d(2026, 10, 28));
    let mut g = Drag::move_event(&e, Pos::Month(d(2026, 10, 27)));
    g.update(Pos::Month(d(2026, 10, 30)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 30, 0, 0), dt(2026, 11, 1, 0, 0)));
}

#[test]
fn month_chip_move_multi_day_timed_keeps_duration() {
    let e = Event::timed(4, "x", 0, dt(2026, 10, 28, 22, 0), dt(2026, 10, 29, 2, 0));
    let mut g = Drag::move_event(&e, Pos::Month(d(2026, 10, 28)));
    g.update(Pos::Month(d(2026, 11, 1)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 11, 1, 22, 0), dt(2026, 11, 2, 2, 0)));
}

// ---- resize

#[test]
fn resize_bottom_snaps_and_keeps_start() {
    let e = timed();
    let mut g = Drag::resize_bottom(&e, Pos::Grid(d(2026, 10, 28), min(11, 0))).unwrap();
    g.update(Pos::Grid(d(2026, 10, 28), min(12, 38)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (e.start, dt(2026, 10, 28, 12, 45)));
    assert!(g.moved());
}

#[test]
fn resize_bottom_enforces_min_15_minutes() {
    let e = timed();
    let mut g = Drag::resize_bottom(&e, Pos::Grid(d(2026, 10, 28), min(11, 0))).unwrap();
    g.update(Pos::Grid(d(2026, 10, 28), min(8, 0)));
    assert_eq!(g.preview().end, dt(2026, 10, 28, 10, 15));
}

#[test]
fn resize_only_for_timed_events() {
    let e = Event::all_day(2, "a", 1, d(2026, 10, 28), d(2026, 10, 28));
    assert!(Drag::resize_bottom(&e, Pos::AllDay(d(2026, 10, 28))).is_none());
}

#[test]
fn resize_ignores_non_grid_positions() {
    let e = timed();
    let mut g = Drag::resize_bottom(&e, Pos::Grid(d(2026, 10, 28), min(11, 0))).unwrap();
    g.update(Pos::AllDay(d(2026, 10, 28)));
    assert_eq!(g.preview().end, e.end);
}

#[test]
fn resize_into_next_day_via_midnight() {
    let e = timed();
    let mut g = Drag::resize_bottom(&e, Pos::Grid(d(2026, 10, 28), min(11, 0))).unwrap();
    g.update(Pos::Grid(d(2026, 10, 29), min(1, 0)));
    assert_eq!(g.preview().end, dt(2026, 10, 29, 1, 0));
}

#[test]
fn apply_preserves_identity_fields() {
    let e = Event::timed(9, "keep", 2, dt(2026, 10, 28, 10, 0), dt(2026, 10, 28, 11, 0));
    let mut g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(10, 0)));
    g.update(Pos::Grid(d(2026, 10, 28), min(12, 0)));
    let out = g.apply(&e);
    assert_eq!((out.id, out.title.as_str(), out.calendar), (9, "keep", 2));
    assert_eq!(out.start, dt(2026, 10, 28, 12, 0));
}

// ---- review round 1

#[test]
fn timed_multi_day_pressed_in_all_day_row_keeps_times() {
    let e = Event::timed(4, "x", 0, dt(2026, 10, 28, 22, 0), dt(2026, 10, 29, 2, 0));
    let mut g = Drag::move_event(&e, Pos::AllDay(d(2026, 10, 28)));
    g.update(Pos::AllDay(d(2026, 10, 30)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 30, 22, 0), dt(2026, 10, 31, 2, 0), false));
}

#[test]
fn moved_month_same_and_different_dates() {
    let e = timed();
    let mut g = Drag::move_event(&e, Pos::Month(d(2026, 10, 28)));
    assert!(!g.moved());
    g.update(Pos::Month(d(2026, 10, 28)));
    assert!(!g.moved());
    g.update(Pos::Month(d(2026, 10, 29)));
    assert!(g.moved());
}

#[test]
fn moved_all_day_row_and_month_same_date_is_not_moved() {
    let e = Event::all_day(2, "a", 1, d(2026, 10, 28), d(2026, 10, 28));
    let mut g = Drag::move_event(&e, Pos::AllDay(d(2026, 10, 28)));
    g.update(Pos::Month(d(2026, 10, 28)));
    assert!(!g.moved());
}

#[test]
fn moved_grid_to_all_day_row_same_date_is_moved() {
    let mut g = Drag::move_event(&timed(), Pos::Grid(d(2026, 10, 28), min(10, 0)));
    g.update(Pos::AllDay(d(2026, 10, 28)));
    assert!(g.moved());
}

#[test]
fn moved_snap_cell_boundaries() {
    let day = d(2026, 10, 28);
    let mut g = Drag::move_event(&timed(), Pos::Grid(day, min(10, 14)));
    g.update(Pos::Grid(day, min(10, 16)));
    assert!(g.moved()); // 10:00 cell -> 10:15 cell
    let mut g = Drag::move_event(&timed(), Pos::Grid(day, min(10, 20)));
    g.update(Pos::Grid(day, min(10, 29)));
    assert!(!g.moved()); // same 10:15 cell
    g.update(Pos::Grid(day, min(10, 30)));
    assert!(g.moved());
}

// Preview is a step function: identity until the pointer leaves the press cell (click
// threshold), then follows the pointer. Deliberate; pinned here.
#[test]
fn move_preview_is_identity_inside_press_cell_then_steps() {
    let e = timed(); // 10:00-11:00
    let day = d(2026, 10, 28);
    let mut g = Drag::move_event(&e, Pos::Grid(day, min(10, 20)));
    g.update(Pos::Grid(day, min(10, 29)));
    assert_eq!(g.preview().start, e.start);
    g.update(Pos::Grid(day, min(10, 30)));
    assert_eq!(g.preview().start, dt(2026, 10, 28, 10, 15));
}

#[test]
fn resize_preview_is_identity_inside_press_cell_then_steps() {
    let e = timed();
    let day = d(2026, 10, 28);
    let mut g = Drag::resize_bottom(&e, Pos::Grid(day, min(11, 0))).unwrap();
    g.update(Pos::Grid(day, min(11, 8)));
    assert_eq!(g.preview().end, e.end);
    g.update(Pos::Grid(day, min(11, 15)));
    assert_eq!(g.preview().end, dt(2026, 10, 28, 11, 15));
}

// ---- end (commit) / cancel

#[test]
fn end_is_none_for_click_on_every_mode() {
    let e = timed();
    let day = d(2026, 10, 28);
    assert!(Drag::create(Pos::Grid(day, min(9, 0))).end().is_none());
    assert!(Drag::create(Pos::Month(day)).end().is_none());
    assert!(Drag::move_event(&e, Pos::Grid(day, min(10, 20))).end().is_none());
    assert!(Drag::resize_bottom(&e, Pos::Grid(day, min(11, 0))).unwrap().end().is_none());
}

#[test]
fn end_is_none_after_returning_to_press_cell() {
    let day = d(2026, 10, 28);
    let mut g = Drag::move_event(&timed(), Pos::Grid(day, min(10, 20)));
    g.update(Pos::Grid(day, min(12, 0)));
    g.update(Pos::Grid(day, min(10, 22)));
    assert!(g.end().is_none());
}

#[test]
fn end_commits_preview_when_moved() {
    let e = timed();
    let day = d(2026, 10, 28);
    let mut g = Drag::move_event(&e, Pos::Grid(day, min(10, 0)));
    g.update(Pos::Grid(day, min(12, 0)));
    let want = g.preview();
    assert_eq!(g.end(), Some(want));
    assert_eq!(want.start, dt(2026, 10, 28, 12, 0));

    let mut g = Drag::create(Pos::Month(day));
    g.update(Pos::Month(d(2026, 10, 30)));
    let p = g.end().unwrap();
    assert!(p.all_day);
    assert_eq!(p.end, dt(2026, 10, 31, 0, 0));
}

#[test]
fn cancel_by_dropping_leaves_event_untouched() {
    let e = timed();
    let day = d(2026, 10, 28);
    let mut slot = Some(Drag::move_event(&e, Pos::Grid(day, min(10, 0))));
    slot.as_mut().unwrap().update(Pos::Grid(day, min(12, 0)));
    assert!(slot.as_ref().unwrap().moved());
    drop(slot.take()); // Esc: no end(), nothing committed
    assert!(slot.is_none());
    assert_eq!(e, timed());
}

#[test]
fn create_dragged_to_earlier_day_clamps_to_start_of_anchor_day() {
    let mut g = Drag::create(Pos::Grid(d(2026, 10, 28), min(9, 0)));
    g.update(Pos::Grid(d(2026, 10, 27), min(15, 0)));
    let p = g.preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 0, 0), dt(2026, 10, 28, 9, 0)));
}

// ---- review round 2

#[test]
fn create_released_on_other_surface_ends_as_cancel() {
    let day = d(2026, 10, 28);
    for (press, cur) in [
        (Pos::Grid(day, min(9, 0)), Pos::AllDay(day)),
        (Pos::Grid(day, min(9, 0)), Pos::Month(d(2026, 10, 30))),
        (Pos::AllDay(day), Pos::Grid(day, min(9, 0))),
        (Pos::Month(day), Pos::Grid(d(2026, 10, 30), min(9, 0))),
    ] {
        let mut g = Drag::create(press);
        g.update(cur);
        assert!(g.end().is_none(), "{press:?} -> {cur:?}");
    }
}

#[test]
fn create_all_day_row_to_month_is_same_surface() {
    let mut g = Drag::create(Pos::AllDay(d(2026, 10, 28)));
    g.update(Pos::Month(d(2026, 10, 30)));
    let p = g.end().unwrap();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 10, 28, 0, 0), dt(2026, 10, 31, 0, 0), true));
}

#[test]
fn multi_day_timed_grid_to_all_day_row_counts_every_day() {
    let e = Event::timed(5, "x", 0, dt(2026, 10, 28, 22, 0), dt(2026, 10, 30, 2, 0)); // 28, 29, 30
    let mut g = Drag::move_event(&e, Pos::Grid(d(2026, 10, 28), min(22, 0)));
    g.update(Pos::AllDay(d(2026, 11, 2)));
    let p = g.preview();
    assert_eq!((p.start, p.end, p.all_day), (dt(2026, 11, 2, 0, 0), dt(2026, 11, 5, 0, 0), true));
}

// ---- review round 3

#[test]
fn create_pressed_at_grid_bottom_is_last_slot() {
    let day = d(2026, 10, 28);
    let p = Drag::create(Pos::Grid(day, 1440)).preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 23, 45), dt(2026, 10, 29, 0, 0)));
}

#[test]
fn create_press_floors_to_slot_start() {
    let p = Drag::create(Pos::Grid(d(2026, 10, 28), min(9, 8))).preview();
    assert_eq!((p.start, p.end), (dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 9, 15)));
}

#[test]
fn move_and_resize_released_off_grid_end_as_none() {
    let day = d(2026, 10, 28);
    let e = timed();
    let mut g = Drag::resize_bottom(&e, Pos::Grid(day, min(11, 0))).unwrap();
    g.update(Pos::AllDay(day));
    assert!(g.end().is_none(), "resize -> all-day row");
    let mut g = Drag::resize_bottom(&e, Pos::Grid(day, min(11, 0))).unwrap();
    g.update(Pos::Month(d(2026, 10, 30)));
    assert!(g.end().is_none(), "resize -> month cell");
    let mut g = Drag::move_event(&e, Pos::Grid(day, min(10, 20)));
    g.update(Pos::Month(d(2026, 10, 30)));
    assert!(g.end().is_none(), "move grid -> month cell");
}

#[test]
fn move_all_day_and_grid_to_all_day_row_still_commit() {
    let day = d(2026, 10, 28);
    let mut g = Drag::move_event(&timed(), Pos::Grid(day, min(10, 0)));
    g.update(Pos::AllDay(day));
    assert!(g.end().is_some_and(|p| p.all_day)); // timed -> all-day conversion
    let a = Event::all_day(2, "a", 1, day, day);
    let mut g = Drag::move_event(&a, Pos::AllDay(day));
    g.update(Pos::AllDay(d(2026, 10, 30)));
    assert!(g.end().is_some());
}
