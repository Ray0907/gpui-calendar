//! Header and sidebar derived text: title, nav names, next-up banner, mini month.

use std::path::PathBuf;

use chrono::{NaiveDate, NaiveTime};

use crate::store::{Store, WORK};
use crate::ui::model::{Model, View};

fn model_at(h: u32, mi: u32) -> Model {
    let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
    Model::new(Store::default(), None::<PathBuf>, today, NaiveTime::from_hms_opt(h, mi, 0).unwrap(), false)
}

#[test]
fn title_follows_the_anchor_and_navigation_steps_by_view() {
    let mut m = model_at(9, 0);
    assert_eq!(m.title(), (Some("October".to_string()), "2026".to_string()));
    assert_eq!(m.step_name(false), "Previous month");
    m.set_view(View::Week);
    assert_eq!(m.step_name(true), "Next week");
    m.step(1);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 11, 4).unwrap());
    assert_eq!(m.title(), (Some("November".to_string()), "2026".to_string()));
    m.set_view(View::Day);
    assert_eq!(m.step_name(true), "Next day");
    m.step(-1);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());
    m.set_view(View::Year);
    assert_eq!(m.step_name(false), "Previous year");
    m.step(-1);
    assert_eq!(m.title().1, "2025");
    m.go_today();
    assert_eq!(m.anchor, m.today);
}

#[test]
fn next_up_is_the_first_visible_future_timed_event() {
    let m = model_at(17, 35);
    let n = m.next_up().expect("Piano lesson at 18:00");
    assert_eq!((n.title.as_str(), n.relative.as_str(), n.clock.as_str()), ("Piano lesson", "in 25 min", "18:00"));
    assert_eq!(n.label(), "Open event");
    let mut m = model_at(17, 35);
    m.toggle_calendar(crate::store::PERSONAL);
    let n = m.next_up().unwrap();
    assert_ne!(n.title, "Piano lesson", "hidden calendars are skipped");
    assert!(model_at(23, 59).next_up().unwrap().relative != "in 25 min");
}

#[test]
fn mini_month_has_named_day_buttons_and_marks_today() {
    let m = model_at(9, 0);
    let days = m.mini_days();
    assert_eq!(days.len() % 7, 0);
    let today = days.iter().find(|d| d.date == m.today).unwrap();
    assert!(today.is_today && !today.out_of_month);
    assert_eq!(today.name, "October 28");
    assert!(days.iter().any(|d| d.out_of_month));
    assert_eq!(days.iter().filter(|d| d.is_today).count(), 1);
    assert_eq!(m.mini_title(), "October 2026");
}

#[test]
fn mini_month_navigation_is_independent_of_the_header() {
    let mut m = model_at(9, 0);
    m.mini_step(1);
    assert_eq!(m.mini_title(), "November 2026");
    assert_eq!(m.anchor, m.today);
    m.pick_day(NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 11, 12).unwrap());
    assert_eq!(m.mini_title(), "November 2026");
    m.set_view(View::Month);
    m.step(1);
    assert_eq!(m.mini_title(), "December 2026", "the mini month follows the header");
    assert!(m.store.calendars[WORK as usize].visible);
}

#[test]
fn stepping_months_does_not_drift_the_day_of_month() {
    let mut m = model_at(9, 0);
    m.pick_day(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
    m.step(1);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    m.step(1);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(), "31 comes back after February");
    m.step(-2);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
    m.set_view(View::Year);
    m.pick_day(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    m.step(1);
    m.step(-1);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
}

#[test]
fn changing_view_or_picking_a_day_restarts_the_anchor() {
    let mut m = model_at(9, 0);
    m.pick_day(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap());
    m.step(1); // Feb 28
    m.set_view(View::Week);
    m.step(-1);
    assert_eq!(m.anchor, NaiveDate::from_ymd_opt(2026, 2, 21).unwrap(), "the new view starts from what was shown");
    m.go_today();
    assert_eq!(m.anchor, m.today);
    m.step(1);
    m.step(-1);
    assert_eq!(m.anchor, m.today);
}

#[test]
fn the_clock_moves_today_and_reports_a_date_change() {
    let mut m = model_at(23, 50);
    let at = |d, h, mi| NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, mi, 0).unwrap();
    assert!(!m.set_now(at(28, 23, 59)), "same day: no date change");
    assert_eq!(m.now, NaiveTime::from_hms_opt(23, 59, 0).unwrap());
    assert!(m.set_now(at(29, 0, 1)));
    assert_eq!(m.today, NaiveDate::from_ymd_opt(2026, 10, 29).unwrap());
    assert_eq!(m.now, NaiveTime::from_hms_opt(0, 1, 0).unwrap());
    let days = m.mini_days();
    assert_eq!(days.iter().filter(|d| d.is_today).count(), 1);
    assert!(days.iter().any(|d| d.is_today && d.date == m.today));
}

#[test]
fn a_day_that_is_today_and_selected_draws_both_marks() {
    let m = model_at(9, 0);
    let d = m.mini_days().into_iter().find(|d| d.is_today).unwrap();
    assert!(d.selected && d.accent_fill() && d.ring(), "selected today keeps the circle and the ring");
    let other = m.mini_days().into_iter().find(|d| !d.is_today && !d.selected).unwrap();
    assert!(!other.accent_fill() && !other.ring());
    let mut m = model_at(9, 0);
    m.pick_day(NaiveDate::from_ymd_opt(2026, 10, 12).unwrap());
    let sel = m.mini_days().into_iter().find(|d| d.selected).unwrap();
    assert!(sel.ring() && !sel.accent_fill());
    let today = m.mini_days().into_iter().find(|d| d.is_today).unwrap();
    assert!(today.accent_fill() && !today.ring());
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn stepping_back_from_the_end_of_a_month_restores_the_day() {
    let mut m = model_at(9, 0);
    m.pick_day(d(2026, 3, 31));
    m.step(-1);
    assert_eq!(m.anchor, d(2026, 2, 28));
    m.step(-1);
    assert_eq!(m.anchor, d(2026, 1, 31), "31 comes back after February");
    m.step(1);
    assert_eq!(m.anchor, d(2026, 2, 28));
    assert_eq!(m.mini_title(), "February 2026");
}

#[test]
fn month_steps_cross_a_year_in_both_directions() {
    let mut m = model_at(9, 0);
    m.pick_day(d(2025, 12, 31));
    m.step(2);
    assert_eq!(m.anchor, d(2026, 2, 28));
    assert_eq!(m.title(), (Some("February".to_string()), "2026".to_string()));
    m.step(-14);
    assert_eq!(m.anchor, d(2024, 12, 31), "14 months back from base Dec 2025 + 2 - 14 = Dec 2024");
    m.set_view(View::Day);
    m.pick_day(d(2026, 1, 1));
    m.step(-1);
    assert_eq!(m.anchor, d(2025, 12, 31));
    m.step(2);
    assert_eq!(m.anchor, d(2026, 1, 2));
}

#[test]
fn set_view_restarts_steps_from_what_is_shown() {
    let mut m = model_at(9, 0);
    m.pick_day(d(2026, 1, 31));
    m.step(1); // Feb 28 shown
    m.set_view(View::Month); // same view again: Feb 28 is shown, the 31st is still wanted
    m.step(1);
    assert_eq!(m.anchor, d(2026, 3, 31), "the 31st is remembered across a view change");
    m.step(-1);
    assert_eq!(m.anchor, d(2026, 2, 28));
    m.set_view(View::Year);
    assert_eq!(m.anchor, d(2026, 2, 28), "switching view never moves the anchor");
    m.step(1);
    assert_eq!(m.anchor, d(2027, 2, 28));
}

#[test]
fn week_to_month_keeps_the_shown_date() {
    let mut m = model_at(9, 0);
    m.set_view(View::Week);
    m.step(1);
    m.step(1);
    m.step(1);
    assert_eq!(m.anchor, d(2026, 11, 18));
    m.set_view(View::Month);
    assert_eq!(m.title(), (Some("November".to_string()), "2026".to_string()));
    assert_eq!(m.mini_title(), "November 2026");
    m.step(1);
    assert_eq!(m.anchor, d(2026, 12, 18));
}

#[test]
fn midnight_moves_today_and_follows_only_a_viewer_who_was_on_today() {
    let at = |y, mo, day, h, mi| d(y, mo, day).and_hms_opt(h, mi, 0).unwrap();
    let mut m = model_at(23, 50);
    m.set_view(View::Day);
    assert!(m.set_now(at(2026, 10, 29, 0, 1)));
    let new = d(2026, 10, 29);
    assert_eq!((m.today, m.anchor), (new, new), "anchor follows today");
    m.step(1);
    assert_eq!(m.anchor, d(2026, 10, 30), "base moved too: stepping starts from the new day");
    m.step(-1);
    assert_eq!(m.anchor, new);
    // across a month end the mini month follows as well
    let mut m = Model::new(Store::default(), None, d(2026, 10, 31), NaiveTime::from_hms_opt(23, 59, 0).unwrap(), false);
    assert!(m.set_now(at(2026, 11, 1, 0, 0)));
    assert_eq!((m.anchor, m.mini_title().as_str()), (d(2026, 11, 1), "November 2026"));
    // a viewer who navigated elsewhere stays there
    let mut m = model_at(23, 50);
    m.pick_day(d(2026, 3, 5));
    m.mini_step(2);
    m.set_now(at(2026, 10, 29, 0, 1));
    assert_eq!(m.today, d(2026, 10, 29));
    assert_eq!(m.anchor, d(2026, 3, 5));
    assert_eq!(m.mini_title(), "May 2026");
}

#[test]
fn a_view_round_trip_does_not_re_base_on_the_clamped_day() {
    let mut m = model_at(9, 0);
    m.pick_day(d(2026, 1, 31));
    m.step(1);
    assert_eq!(m.anchor, d(2026, 2, 28));
    m.set_view(View::Week);
    assert_eq!(m.anchor, d(2026, 2, 28), "switching view never moves the anchor");
    m.set_view(View::Month);
    m.step(-1);
    assert_eq!(m.anchor, d(2026, 1, 31), "back to January keeps the 31st");
    m.step(2);
    assert_eq!(m.anchor, d(2026, 3, 31));
    // A week step is a real choice of day: the 31st is no longer wanted.
    m.set_view(View::Week);
    m.step(1);
    assert_eq!(m.anchor, d(2026, 4, 7));
    m.set_view(View::Month);
    m.step(1);
    assert_eq!(m.anchor, d(2026, 5, 7));
}
