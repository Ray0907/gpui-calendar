use chrono::{NaiveDate, Weekday};

use crate::domain::event::Event;
use crate::domain::month::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn at(m: u32, day: u32, h: u32) -> chrono::NaiveDateTime {
    d(2026, m, day).and_hms_opt(h, 0, 0).unwrap()
}

fn weeks() -> Weeks {
    Weeks::around(d(2026, 10, 28), Weekday::Mon, 10)
}

#[test]
fn weeks_are_a_finite_run_centered_on_the_start_date() {
    let w = weeks();
    assert_eq!(w.len(), 21);
    assert_eq!(w.start_of(10), d(2026, 10, 26));
    assert_eq!(w.start_of(11), d(2026, 11, 2));
    assert_eq!(w.index_of(d(2026, 10, 28)), 10);
    assert_eq!(w.index_of(d(2026, 11, 1)), 10);
    assert_eq!(w.index_of(d(2100, 1, 1)), 20, "clamped to the last week");
    assert_eq!(w.index_of(d(1990, 1, 1)), 0, "clamped to the first week");
    let sun = Weeks::around(d(2026, 10, 28), Weekday::Sun, 2);
    assert_eq!(sun.start_of(2), d(2026, 10, 25));
}

#[test]
fn visible_weeks_follow_the_scroll_offset() {
    let w = weeks();
    assert_eq!(w.visible(0., 300., 100.), 0..3);
    assert_eq!(w.visible(100., 300., 100.), 1..4);
    assert_eq!(w.visible(150., 300., 100.), 1..5);
    assert_eq!(w.visible(1900., 300., 100.), 19..21, "clamped to the run");
    assert_eq!(w.visible(99999., 300., 100.), 21..21);
    assert_eq!(w.visible(-40., 300., 100.), 0..3, "overscroll counts as the top");
}

#[test]
fn the_top_week_is_the_first_one_at_least_half_visible() {
    let w = weeks();
    assert_eq!(w.top_week(0., 100.), 0);
    assert_eq!(w.top_week(50., 100.), 0, "exactly half still counts");
    assert_eq!(w.top_week(51., 100.), 1);
    assert_eq!(w.top_week(150., 100.), 1);
    assert_eq!(w.top_week(151., 100.), 2);
    assert_eq!(w.top_week(-10., 100.), 0);
    assert_eq!(w.top_week(99999., 100.), 20, "clamped to the last week");
}

#[test]
fn a_week_is_named_by_the_month_of_its_last_day() {
    let w = weeks();
    assert_eq!(w.month_of_week(9), (2026, 10)); // Oct 19-25
    assert_eq!(w.month_of_week(10), (2026, 11)); // Oct 26 - Nov 1 ends in November
}

#[test]
fn title_month_follows_the_offset() {
    let w = weeks();
    // Oct 26 - Nov 1 ends in November.
    assert_eq!(w.title_month(1000., 100.), (2026, 11));
    // Oct 19 - 25 ends in October.
    assert_eq!(w.title_month(900., 100.), (2026, 10));
}

#[test]
fn scrolling_to_a_month_names_that_month_for_both_week_starts() {
    for ws in [Weekday::Mon, Weekday::Sun] {
        let w = Weeks::around(d(2026, 6, 15), ws, 200);
        for y in 2025..=2028 {
            for m in 1..=12 {
                let ix = w.first_week_of((y, m));
                assert!(w.start_of(ix) <= d(y, m, 1) && d(y, m, 1) < w.start_of(ix) + chrono::Duration::days(7), "{y}-{m} {ws:?}");
                assert_eq!(w.title_month(ix as f32 * 128., 128.), (y, m), "{y}-{m} {ws:?}");
            }
        }
    }
}

#[test]
fn fully_visible_weeks_leave_out_the_cut_ones() {
    let w = Weeks::around(d(2026, 10, 28), Weekday::Mon, 10);
    assert_eq!(w.visible(150., 300., 100.), 1..5, "touching");
    assert_eq!(w.fully_visible(150., 300., 100.), 2..4, "whole rows only");
    assert_eq!(w.fully_visible(200., 300., 100.), 2..5);
    assert_eq!(w.fully_visible(150., 50., 100.), 2..2, "none fits");
    assert_eq!(w.fully_visible(1900., 300., 100.), 19..21, "clamped to the run");
}

#[test]
fn the_last_top_week_depends_on_the_viewport() {
    let w = weeks();
    assert_eq!(w.last_top(0., 100.), 20, "before layout nothing is known: the last week");
    assert_eq!(w.last_top(5000., 100.), 0, "a viewport taller than the run");
}

#[test]
fn a_run_knows_its_week_start() {
    assert_eq!(weeks().week_start(), Weekday::Mon);
    assert_eq!(Weeks::around(d(2026, 10, 28), Weekday::Sun, 2).week_start(), Weekday::Sun);
}

#[test]
fn a_span_is_the_weeks_an_event_touches_clipped_to_the_run() {
    let w = weeks(); // Aug 17, 2026 - Jan 3, 2027
    assert_eq!(w.span(&Event::all_day(1, "x", 0, d(2026, 10, 30), d(2026, 11, 3)).days()), 10..12);
    assert_eq!(w.span(&Event::all_day(1, "x", 0, d(2026, 10, 28), d(2026, 10, 28)).days()), 10..11);
    assert_eq!(w.span(&Event::all_day(1, "x", 0, d(2026, 8, 14), d(2026, 8, 18)).days()), 0..1, "starts before the run");
    assert_eq!(w.span(&Event::all_day(1, "x", 0, d(2026, 8, 1), d(2026, 8, 10)).days()).len(), 0, "wholly before");
    assert_eq!(w.span(&Event::all_day(1, "x", 0, d(2027, 3, 1), d(2027, 3, 2)).days()).len(), 0, "wholly after");
}

#[test]
fn bucketing_gives_each_week_the_events_it_would_have_found_by_scanning() {
    let w = weeks();
    let events: Vec<Event> = (0..2000u64)
        .map(|i| {
            let day = d(2026, 8, 17) + chrono::Duration::days((i * 7 % 140) as i64);
            if i % 50 == 0 { Event::all_day(i, "bar", 0, day, day + chrono::Duration::days(9)) } else { Event::timed(i, "x", 0, day.and_hms_opt(9, 0, 0).unwrap(), day.and_hms_opt(10, 0, 0).unwrap()) }
        })
        .collect();
    let refs: Vec<&Event> = events.iter().collect();
    let buckets = bucket_by_week(&w, &events);
    for ix in 0..w.len() {
        let mine: Vec<&Event> = buckets.get(&ix).map_or(&[][..], Vec::as_slice).iter().map(|&i| &events[i]).collect();
        let (a, b) = (layout_week(w.start_of(ix), &mine, 3), layout_week(w.start_of(ix), &refs, 3));
        let ids = |l: &WeekLayout| l.items.iter().chain(&l.hidden).map(|p| (p.id, p.lane, p.col, p.span)).collect::<Vec<_>>();
        assert_eq!((ids(&a), a.more), (ids(&b), b.more), "week {ix}");
    }
    // Each callback now looks at every event once, not once per visible week.
    let entries: usize = buckets.values().map(Vec::len).sum();
    let bars = events.iter().filter(|e| e.all_day).count();
    assert!(entries < events.len() + bars * 3, "{entries} entries for {} events", events.len());
    assert!(buckets.values().all(|b| b.len() < 200), "a week holds its share, not everything");
}

fn name(e: &Event, from_prev: bool, to_next: bool) -> String {
    let p = Placed { id: e.id, ix: 0, lane: 0, col: 0, span: 1, from_prev, to_next, pill: false };
    item_name(e, "Work", true, &p)
}

#[test]
fn a_chip_name_keeps_the_title_time_and_calendar() {
    assert_eq!(name(&Event::timed(1, "Standup", 0, at(10, 28, 9), at(10, 28, 10)), false, false), "Standup, 09:00, Work");
    assert_eq!(name(&Event::all_day(1, "Offsite", 0, d(2026, 10, 28), d(2026, 10, 28)), false, false), "Offsite, Work, all day");
}

#[test]
fn a_bar_name_has_its_date_range_and_which_segment_it_is() {
    let trip = Event::all_day(1, "Trip", 0, d(2026, 10, 30), d(2026, 11, 3));
    assert_eq!(name(&trip, false, true), "Trip, Work, all day, Oct 30 to Nov 3, continues into the next week");
    assert_eq!(name(&trip, true, false), "Trip, Work, all day, Oct 30 to Nov 3, continues from the previous week");
    let long = Event::all_day(1, "Trip", 0, d(2026, 10, 20), d(2026, 11, 10));
    assert_eq!(name(&long, true, true), "Trip, Work, all day, Oct 20 to Nov 10, continues from the previous week, continues into the next week");
}

#[test]
fn a_timed_event_over_midnight_names_its_end_date() {
    let deploy = Event::timed(1, "Deploy", 0, at(10, 30, 22), at(10, 31, 1));
    assert_eq!(name(&deploy, false, false), "Deploy, 22:00 Oct 30 to 01:00 Oct 31, Work");
}

fn ev(id: u64, e: Event) -> Event {
    Event { id, ..e }
}

fn lay(events: &[Event], start: NaiveDate, cap: usize) -> WeekLayout {
    let refs: Vec<&Event> = events.iter().collect();
    layout_week(start, &refs, cap)
}

fn item(l: &WeekLayout, id: u64) -> Placed {
    *l.items.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("event {id} not drawn"))
}

#[test]
fn the_first_row_is_the_same_lane_in_every_cell() {
    let events = [
        ev(1, Event::timed(0, "a", 0, at(10, 26, 9), at(10, 26, 10))),
        ev(2, Event::timed(0, "b", 1, at(10, 28, 9), at(10, 28, 10))),
        ev(3, Event::timed(0, "c", 2, at(10, 31, 9), at(10, 31, 10))),
    ];
    let l = lay(&events, d(2026, 10, 26), 3);
    assert_eq!([item(&l, 1).lane, item(&l, 2).lane, item(&l, 3).lane], [0, 0, 0]);
    assert_eq!([item(&l, 1).col, item(&l, 2).col, item(&l, 3).col], [0, 2, 5]);
    assert_eq!(l.more, [0; 7]);
}

#[test]
fn bars_come_first_and_keep_one_lane_across_their_days() {
    let events = [
        ev(1, Event::timed(0, "chip", 0, at(10, 28, 8), at(10, 28, 9))),
        ev(2, Event::all_day(0, "conf", 0, d(2026, 10, 27), d(2026, 10, 29))),
    ];
    let l = lay(&events, d(2026, 10, 26), 3);
    let bar = item(&l, 2);
    assert_eq!((bar.lane, bar.col, bar.span, bar.pill), (0, 1, 3, true));
    let chip = item(&l, 1);
    assert_eq!((chip.lane, chip.col, chip.span, chip.pill), (1, 2, 1, false), "under the bar that covers its day");
}

#[test]
fn a_chip_outside_a_bar_reuses_lane_zero() {
    let events = [
        ev(1, Event::all_day(0, "conf", 0, d(2026, 10, 27), d(2026, 10, 28))),
        ev(2, Event::timed(0, "chip", 0, at(10, 30, 8), at(10, 30, 9))),
    ];
    let l = lay(&events, d(2026, 10, 26), 3);
    assert_eq!(item(&l, 2).lane, 0);
}

#[test]
fn a_timed_event_over_midnight_is_a_two_day_bar() {
    let events = [ev(1, Event::timed(0, "deploy", 0, at(10, 30, 22), at(10, 31, 1)))];
    let p = item(&lay(&events, d(2026, 10, 26), 3), 1);
    assert_eq!((p.col, p.span, p.pill), (4, 2, true));
}

#[test]
fn a_bar_is_cut_at_the_week_edges_and_says_so() {
    let events = [ev(1, Event::all_day(0, "trip", 0, d(2026, 10, 30), d(2026, 11, 3)))];
    let a = item(&lay(&events, d(2026, 10, 26), 3), 1);
    assert_eq!((a.col, a.span, a.from_prev, a.to_next), (4, 3, false, true));
    let b = item(&lay(&events, d(2026, 11, 2), 3), 1);
    assert_eq!((b.col, b.span, b.from_prev, b.to_next), (0, 2, true, false));
    assert!(lay(&events, d(2026, 11, 9), 3).items.is_empty());
    assert!(lay(&events, d(2026, 10, 19), 3).items.is_empty());
}

#[test]
fn rows_past_the_cap_become_a_count_per_day() {
    let mut events: Vec<Event> = (1..=5).map(|i| ev(i, Event::timed(0, "x", 0, at(10, 28, 8 + i as u32), at(10, 28, 9 + i as u32)))).collect();
    events.push(ev(9, Event::all_day(0, "bar", 1, d(2026, 10, 28), d(2026, 10, 29))));
    let l = lay(&events, d(2026, 10, 26), 3);
    assert_eq!(l.items.len(), 3);
    assert!(l.items.iter().all(|p| p.lane < 3));
    assert_eq!(l.more, [0, 0, 3, 0, 0, 0, 0], "six on the 28th, three shown");
    assert_eq!(item(&l, 9).lane, 0, "the bar keeps its row");
    // Two rows only: one more hidden.
    assert_eq!(lay(&events, d(2026, 10, 26), 2).more[2], 4);
}

#[test]
fn a_hidden_bar_counts_on_each_day_it_covers() {
    let events = [
        ev(1, Event::timed(0, "a", 0, at(10, 27, 8), at(10, 27, 9))),
        ev(2, Event::timed(0, "b", 0, at(10, 28, 8), at(10, 28, 9))),
        ev(3, Event::all_day(0, "bar", 0, d(2026, 10, 27), d(2026, 10, 29))),
    ];
    let l = lay(&events, d(2026, 10, 26), 1);
    assert_eq!(l.items.len(), 1);
    assert_eq!(l.more, [0, 1, 1, 0, 0, 0, 0]);
}

#[test]
fn the_grid_is_the_window_less_the_page_chrome_and_the_sidebar_or_rail() {
    assert_eq!(grid_width(1440., false), 1156.);
    assert_eq!(grid_width(1120., false), WIDE_GRID);
    assert_eq!(grid_width(960., true), 848.);
    assert_eq!(cap_for(grid_width(1120., false)), 3);
    assert_eq!(cap_for(grid_width(900., true)), 2);
}

#[test]
fn cap_is_two_on_a_narrow_grid_and_three_on_a_wide_one() {
    assert_eq!(cap_for(WIDE_GRID - 1.), 2);
    assert_eq!(cap_for(WIDE_GRID), 3);
    assert_eq!(cap_for(1156.), 3);
    assert_eq!(cap_for(600.), 2);
}

#[test]
fn arrows_walk_events_in_reading_order_and_stop_at_the_ends() {
    let events = [
        ev(7, Event::timed(0, "late", 0, at(10, 29, 9), at(10, 29, 10))),
        ev(3, Event::timed(0, "early", 0, at(10, 27, 9), at(10, 27, 10))),
        ev(5, Event::all_day(0, "day", 0, d(2026, 10, 27), d(2026, 10, 27))),
    ];
    let refs: Vec<&Event> = events.iter().collect();
    let order = reading_order(&refs);
    assert_eq!(order, vec![5, 3, 7], "by start, ties by id");
    assert_eq!(neighbor(&order, 5, true), Some(3));
    assert_eq!(neighbor(&order, 3, false), Some(5));
    assert_eq!(neighbor(&order, 7, true), Some(7));
    assert_eq!(neighbor(&order, 5, false), Some(5));
    assert_eq!(neighbor(&order, 99, true), None, "a gone event is not in the walk");
}

#[test]
fn from_nowhere_forward_enters_at_the_month_on_screen_and_backward_leaves_it_from_the_end() {
    let events = [
        ev(1, Event::timed(0, "sep", 0, at(9, 30, 9), at(9, 30, 10))),
        ev(2, Event::timed(0, "oct early", 0, at(10, 5, 9), at(10, 5, 10))),
        ev(3, Event::timed(0, "oct late", 0, at(10, 20, 9), at(10, 20, 10))),
        ev(4, Event::timed(0, "nov", 0, at(11, 2, 9), at(11, 2, 10))),
    ];
    let refs: Vec<&Event> = events.iter().collect();
    assert_eq!(enter(&refs, (2026, 10), true), Some(2), "first event of the month, not of all time");
    assert_eq!(enter(&refs, (2026, 10), false), Some(3), "up and left start from the other end");
    assert_eq!(enter(&refs, (2026, 12), true), None);
    assert_eq!(enter(&refs, (2026, 12), false), Some(4), "nothing in the month: the nearest before it");
    assert_eq!(enter(&refs, (2026, 8), false), None);
    assert_eq!(enter(&[], (2026, 10), true), None);
}

#[test]
fn the_pieces_of_one_bar_share_a_lane_in_every_week() {
    // A (Oct 30 - Nov 4) is cut by the week edge; D starts Monday Nov 2 and is longer than A's second piece.
    let events = [
        ev(1, Event::all_day(0, "A", 0, d(2026, 10, 30), d(2026, 11, 4))),
        ev(2, Event::all_day(0, "D", 0, d(2026, 11, 2), d(2026, 11, 10))),
    ];
    let (first, second) = (lay(&events, d(2026, 10, 26), 3), lay(&events, d(2026, 11, 2), 3));
    assert_eq!(item(&first, 1).lane, item(&second, 1).lane, "A keeps its row across the week edge");
    assert_eq!((item(&second, 1).lane, item(&second, 2).lane), (0, 1), "the bar that began earlier holds the upper row");
}

#[test]
fn items_past_the_cap_are_listed_as_hidden() {
    let events: Vec<Event> = (1..=4).map(|i| ev(i, Event::timed(0, "x", 0, at(10, 28, 7 + i as u32), at(10, 28, 8 + i as u32)))).collect();
    let l = lay(&events, d(2026, 10, 26), 2);
    assert_eq!(l.hidden.iter().map(|p| (p.id, p.col)).collect::<Vec<_>>(), [(3, 2), (4, 2)]);
    assert!(lay(&events, d(2026, 10, 26), 4).hidden.is_empty());
}
