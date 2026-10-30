//! The mini-calendar month picker's state: no GPUI.

use chrono::{Datelike, NaiveDate, NaiveTime};

use crate::store::Store;
use crate::ui::model::{Model, Screen, View, month_cursor};

fn model() -> Model {
    let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
    Model::new(Store::default(), None, today, NaiveTime::from_hms_opt(17, 35, 0).unwrap(), false)
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn picking_a_month_moves_the_mini_month_and_the_view_to_the_same_date() {
    let mut m = model();
    m.pick_month(2027, 3);
    assert_eq!(m.anchor, d(2027, 3, 28));
    assert_eq!(m.mini_title(), "March 2027");
    assert!(m.mini_days().iter().any(|x| x.selected && x.date == d(2027, 3, 28)));
}

#[test]
fn the_day_of_the_month_is_clamped_and_a_long_day_is_remembered() {
    let mut m = model();
    m.pick_month(2027, 2);
    assert_eq!(m.anchor, d(2027, 2, 28));
    m.pick_month(2026, 10);
    m.pick_day(d(2026, 10, 31));
    m.pick_month(2027, 2);
    assert_eq!(m.anchor, d(2027, 2, 28), "clamped to the last day");
    m.pick_month(2027, 3);
    assert_eq!(m.anchor, d(2027, 3, 31), "the 31st is still wanted");
}

#[test]
fn picking_a_month_leaves_appearance_and_keeps_the_view() {
    let mut m = model();
    m.set_view(View::Week);
    m.toggle_appearance();
    m.pick_month(2026, 12);
    assert_eq!(m.screen, Screen::Calendar);
    assert_eq!(m.view, View::Week);
    assert_eq!(m.anchor.month(), 12);
}

#[test]
fn the_picker_starts_on_the_month_the_mini_calendar_shows() {
    let mut m = model();
    assert_eq!(m.mini_ym(), (2026, 10));
    m.mini_step(3);
    assert_eq!(m.mini_ym(), (2027, 1));
    assert_eq!(m.anchor.month(), 10, "the arrows move the mini month only");
}

#[test]
fn arrows_move_by_one_and_three_and_wrap_across_the_year_edges() {
    let at = |ym, key| month_cursor(ym, key);
    assert_eq!(at((2026, 10), "right"), Some((2026, 11)));
    assert_eq!(at((2026, 12), "right"), Some((2027, 1)));
    assert_eq!(at((2026, 1), "left"), Some((2025, 12)));
    assert_eq!(at((2026, 10), "down"), Some((2027, 1)));
    assert_eq!(at((2026, 2), "up"), Some((2025, 11)));
    assert_eq!(at((2026, 5), "up"), Some((2026, 2)));
    assert_eq!(at((2026, 5), "tab"), None);
}

#[test]
fn home_and_end_go_to_the_first_and_last_month_of_the_year() {
    assert_eq!(month_cursor((2026, 7), "home"), Some((2026, 1)));
    assert_eq!(month_cursor((2026, 7), "end"), Some((2026, 12)));
}

#[test]
fn the_cursor_stops_at_the_first_and_last_year() {
    assert_eq!(month_cursor((9999, 12), "right"), Some((9999, 12)));
    assert_eq!(month_cursor((9999, 11), "down"), Some((9999, 11)));
    assert_eq!(month_cursor((1, 1), "left"), Some((1, 1)));
    assert_eq!(month_cursor((1, 2), "up"), Some((1, 2)));
    assert_eq!(month_cursor((9999, 11), "right"), Some((9999, 12)));
}

#[test]
fn picking_a_year_out_of_range_lands_on_the_nearest_year() {
    let mut m = model();
    m.pick_month(300_000, 5);
    assert_eq!(m.anchor.year(), 9999);
    m.pick_month(-4, 5);
    assert_eq!(m.anchor.year(), 1);
}
