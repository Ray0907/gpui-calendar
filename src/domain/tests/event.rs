use std::collections::HashSet;

use chrono::{NaiveDate, NaiveDateTime};

use crate::domain::dates::{DateRange, week_range};
use crate::domain::event::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
fn dt(y: i32, m: u32, day: u32, h: u32, mi: u32) -> NaiveDateTime {
    d(y, m, day).and_hms_opt(h, mi, 0).unwrap()
}

#[test]
fn timed_same_day() {
    let e = Event::timed(1, "a", 0, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 30));
    assert_eq!(e.days().len(), 1);
    assert!(e.occupies(d(2026, 10, 28)));
    assert!(!e.occupies(d(2026, 10, 29)));
    assert_eq!(e.segment_on(d(2026, 10, 28)), Some((540, 630)));
    assert_eq!(e.segment_on(d(2026, 10, 29)), None);
    assert!(!e.in_all_day_row());
}

#[test]
fn timed_crossing_midnight_has_two_segments() {
    let e = Event::timed(1, "late", 0, dt(2026, 10, 28, 22, 0), dt(2026, 10, 29, 1, 30));
    assert_eq!(e.days().len(), 2);
    assert_eq!(e.segment_on(d(2026, 10, 28)), Some((1320, 1440)));
    assert_eq!(e.segment_on(d(2026, 10, 29)), Some((0, 90)));
    // routing: timed events render only as grid segments, never in the all-day row
    assert!(!e.in_all_day_row());
}

#[test]
fn timed_ending_exactly_midnight_stays_one_day() {
    let e = Event::timed(1, "x", 0, dt(2026, 10, 28, 22, 0), dt(2026, 10, 29, 0, 0));
    assert_eq!(e.days().len(), 1);
    assert!(!e.occupies(d(2026, 10, 29)));
    assert_eq!(e.segment_on(d(2026, 10, 28)), Some((1320, 1440)));
    assert!(!e.in_all_day_row());
}

#[test]
fn timed_spanning_three_days_has_full_middle_day() {
    let e = Event::timed(1, "x", 0, dt(2026, 10, 28, 12, 0), dt(2026, 10, 30, 6, 0));
    assert_eq!(e.segment_on(d(2026, 10, 29)), Some((0, 1440)));
    assert_eq!(e.days().len(), 3);
}

#[test]
fn zero_or_negative_length_is_clamped_to_15_minutes() {
    let e = Event::timed(1, "x", 0, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 9, 0));
    assert_eq!(e.end, dt(2026, 10, 28, 9, 15));
    let e = Event::timed(1, "x", 0, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 8, 0));
    assert_eq!(e.end, dt(2026, 10, 28, 9, 15));
}

#[test]
fn all_day_multi_day() {
    let e = Event::all_day(2, "trip", 1, d(2026, 10, 30), d(2026, 11, 2));
    assert_eq!(e.start, dt(2026, 10, 30, 0, 0));
    assert_eq!(e.end, dt(2026, 11, 3, 0, 0));
    assert_eq!(e.days().len(), 4);
    assert!(e.occupies(d(2026, 11, 2)));
    assert!(!e.occupies(d(2026, 11, 3)));
    assert_eq!(e.segment_on(d(2026, 10, 31)), None);
    assert!(e.in_all_day_row());
}

#[test]
fn all_day_last_before_first_is_single_day() {
    let e = Event::all_day(2, "x", 1, d(2026, 10, 30), d(2026, 10, 1));
    assert_eq!(e.days().len(), 1);
}

#[test]
fn range_and_day_queries_include_spanning_events() {
    let evs = vec![
        Event::all_day(1, "trip", 0, d(2026, 10, 25), d(2026, 10, 27)),
        Event::timed(2, "in", 0, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 0)),
        Event::timed(3, "out", 0, dt(2026, 11, 5, 9, 0), dt(2026, 11, 5, 10, 0)),
        Event::timed(4, "late", 0, dt(2026, 10, 25, 23, 0), dt(2026, 10, 26, 1, 0)),
    ];
    let week = week_range(d(2026, 10, 28), chrono::Weekday::Mon); // 26..Nov 2
    let ids: Vec<u64> = events_in_range(&evs, week).iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![1, 2, 4]);
    let on26: Vec<u64> = events_on_day(&evs, d(2026, 10, 26)).iter().map(|e| e.id).collect();
    assert_eq!(on26, vec![1, 4]);
    let empty = DateRange { start: d(2026, 10, 1), end: d(2026, 10, 1) };
    assert!(events_in_range(&evs, empty).is_empty());
}

#[test]
fn visibility_filters_hidden_calendars() {
    let evs = vec![
        Event::timed(1, "w", 0, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 0)),
        Event::timed(2, "p", 1, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 0)),
        Event::timed(3, "s", 2, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 0)),
    ];
    let hidden: HashSet<CalendarId> = [1].into_iter().collect();
    let ids: Vec<u64> = visible(&evs, &hidden).iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![1, 3]);
    assert_eq!(visible(&evs, &HashSet::new()).len(), 3);
    let all: HashSet<CalendarId> = [0, 1, 2].into_iter().collect();
    assert!(visible(&evs, &all).is_empty());
}

#[test]
fn range_boundaries_are_half_open() {
    let evs = vec![
        Event::timed(1, "before", 0, dt(2026, 10, 25, 9, 0), dt(2026, 10, 25, 10, 0)), // day before range
        Event::timed(2, "at end", 0, dt(2026, 11, 2, 9, 0), dt(2026, 11, 2, 10, 0)),   // starts on range.end
        Event::timed(3, "first", 0, dt(2026, 10, 26, 9, 0), dt(2026, 10, 26, 10, 0)),
        Event::timed(4, "last", 0, dt(2026, 11, 1, 9, 0), dt(2026, 11, 1, 10, 0)),
        Event::timed(5, "to midnight", 0, dt(2026, 10, 25, 22, 0), dt(2026, 10, 26, 0, 0)), // ends at range.start
    ];
    let week = week_range(d(2026, 10, 28), chrono::Weekday::Mon); // 26..Nov 2
    let ids: Vec<u64> = events_in_range(&evs, week).iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![3, 4]);
}

#[test]
fn zero_length_event_from_serde_still_shows() {
    let mut e = Event::timed(1, "x", 0, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 0));
    e.end = e.start; // bypasses the constructor, as deserialization does
    assert_eq!(e.days().len(), 1);
    assert_eq!(e.segment_on(d(2026, 10, 28)), Some((540, 555)));
    e.end = dt(2026, 10, 28, 8, 0);
    assert_eq!(e.segment_on(d(2026, 10, 28)), Some((540, 555)));
}

#[test]
fn serde_roundtrip_event_preview_pos() {
    use crate::domain::drag::{Pos, Preview};
    fn rt<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(v: T) {
        let back: T = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(back, v);
    }
    rt(Event::timed(1, "t", 2, dt(2026, 10, 28, 9, 0), dt(2026, 10, 28, 10, 0)));
    rt(Event::all_day(2, "a", 1, d(2026, 10, 28), d(2026, 10, 29)));
    rt(Preview { start: dt(2026, 10, 28, 9, 0), end: dt(2026, 10, 28, 9, 15), all_day: false });
    for p in [Pos::Grid(d(2026, 10, 28), 545), Pos::AllDay(d(2026, 10, 28)), Pos::Month(d(2026, 10, 28))] {
        rt(p);
    }
}
