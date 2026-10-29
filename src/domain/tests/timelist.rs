use chrono::NaiveTime;

use crate::domain::timelist::*;

fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}

#[test]
fn a_start_list_is_the_96_quarter_hours_of_the_day() {
    let rows = start_rows(t(9, 30));
    assert_eq!((rows.len(), rows[0], rows[1], rows[95]), (96, t(0, 0), t(0, 15), t(23, 45)));
}

#[test]
fn a_current_time_off_the_grid_is_a_row_in_its_place() {
    let rows = start_rows(t(9, 50));
    assert_eq!(rows.len(), 97);
    assert_eq!(rows.iter().position(|&r| r == t(9, 50)), rows.iter().position(|&r| r == t(9, 45)).map(|i| i + 1));
}

#[test]
fn the_end_list_starts_at_the_start_time_when_both_are_on_one_day() {
    let rows = end_rows(t(9, 30), t(10, 0), true);
    assert_eq!((rows[0], rows[1], *rows.last().unwrap()), (t(9, 30), t(9, 45), t(23, 45)));
    assert_eq!(rows.len(), 58);
    // An end time that is already before the start has no row: the list never offers a time the form refuses.
    assert!(!end_rows(t(9, 30), t(8, 0), true).contains(&t(8, 0)));
}

#[test]
fn the_end_list_is_the_whole_day_when_the_end_is_on_a_later_day() {
    let rows = end_rows(t(9, 30), t(8, 0), false);
    assert_eq!((rows.len(), rows[0]), (96, t(0, 0)));
}

#[test]
fn durations_read_like_google_calendars() {
    let got: Vec<_> = [0, 15, 30, 45, 60, 75, 120, 135].map(duration).into();
    assert_eq!(got, ["0 min", "15 min", "30 min", "45 min", "1 h", "1 h 15 min", "2 h", "2 h 15 min"]);
}
