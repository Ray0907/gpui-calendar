use chrono::{NaiveDate, NaiveTime, Weekday};

use crate::domain::dates::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn week_start_monday_and_sunday() {
    // 2026-10-28 is a Wednesday
    assert_eq!(week_start_of(d(2026, 10, 28), Weekday::Mon), d(2026, 10, 26));
    assert_eq!(week_start_of(d(2026, 10, 28), Weekday::Sun), d(2026, 10, 25));
    assert_eq!(week_start_of(d(2026, 10, 26), Weekday::Mon), d(2026, 10, 26));
    assert_eq!(week_start_of(d(2026, 10, 25), Weekday::Mon), d(2026, 10, 19));
}

#[test]
fn week_range_is_seven_days() {
    let r = week_range(d(2026, 10, 28), Weekday::Mon);
    assert_eq!((r.start, r.end, r.len()), (d(2026, 10, 26), d(2026, 11, 2), 7));
}

#[test]
fn month_range_lengths_and_leap_year() {
    assert_eq!(month_range(2026, 10).len(), 31);
    assert_eq!(month_range(2024, 2).len(), 29);
    assert_eq!(month_range(2026, 2).len(), 28);
    assert_eq!(month_range(2026, 12).end, d(2027, 1, 1));
}

#[test]
fn month_grid_covers_whole_weeks() {
    // Oct 2026 starts Thursday; ends Saturday the 31st
    let r = month_grid_range(2026, 10, Weekday::Mon);
    assert_eq!(r.start, d(2026, 9, 28));
    assert_eq!(r.end, d(2026, 11, 2));
    assert_eq!(r.len(), 35);
    let r = month_grid_range(2026, 10, Weekday::Sun);
    assert_eq!(r.start, d(2026, 9, 27));
    assert_eq!(r.end, d(2026, 11, 1));
    // Feb 2026 starts on Sunday, exactly 4 weeks with Sunday start
    assert_eq!(month_grid_range(2026, 2, Weekday::Sun).len(), 28);
}

#[test]
fn month_grid_six_weeks() {
    // Aug 2026 starts Saturday and ends Monday the 31st: 6 Monday-start weeks
    let r = month_grid_range(2026, 8, Weekday::Mon);
    assert_eq!((r.start, r.end, r.len()), (d(2026, 7, 27), d(2026, 9, 7), 42));
}

#[test]
fn year_range_handles_leap() {
    assert_eq!(year_range(2024).len(), 366);
    assert_eq!(year_range(2026).len(), 365);
}

#[test]
fn range_contains_and_days() {
    let r = week_range(d(2026, 10, 28), Weekday::Mon);
    assert!(r.contains(d(2026, 10, 26)));
    assert!(!r.contains(d(2026, 11, 2)));
    let v: Vec<_> = r.days().collect();
    assert_eq!(v.len(), 7);
    assert_eq!(v[6], d(2026, 11, 1));
}

#[test]
fn add_months_clamps_day() {
    assert_eq!(add_months(d(2026, 1, 31), 1), d(2026, 2, 28));
    assert_eq!(add_months(d(2024, 1, 31), 1), d(2024, 2, 29));
    assert_eq!(add_months(d(2026, 11, 15), 3), d(2027, 2, 15));
    assert_eq!(add_months(d(2026, 1, 15), -2), d(2025, 11, 15));
    assert_eq!(add_months(d(2026, 5, 10), 0), d(2026, 5, 10));
}

#[test]
fn format_time_12_and_24() {
    let t = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
    assert_eq!(format_time(t(0, 5), true), "00:05");
    assert_eq!(format_time(t(13, 30), true), "13:30");
    assert_eq!(format_time(t(0, 5), false), "12:05 AM");
    assert_eq!(format_time(t(12, 0), false), "12:00 PM");
    assert_eq!(format_time(t(13, 30), false), "1:30 PM");
}

#[test]
fn format_hour_labels() {
    assert_eq!(format_hour(9, true), "09:00");
    assert_eq!(format_hour(0, false), "12 AM");
    assert_eq!(format_hour(12, false), "12 PM");
    assert_eq!(format_hour(15, false), "3 PM");
}
