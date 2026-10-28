//! Week and day state on the model: the shown days and the header title.

use std::path::PathBuf;

use chrono::{NaiveDate, NaiveTime};

use crate::store::Store;
use crate::ui::model::{Model, View};

fn model_on(y: i32, m: u32, d: u32) -> Model {
    let today = NaiveDate::from_ymd_opt(y, m, d).unwrap();
    Model::new(Store::default(), None::<PathBuf>, today, NaiveTime::from_hms_opt(9, 0, 0).unwrap(), false)
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn the_week_title_names_the_months_it_touches() {
    let mut m = model_on(2026, 10, 28);
    m.set_view(View::Week);
    assert_eq!(m.title(), ("Oct – Nov".to_string(), "2026".to_string()), "Oct 26 to Nov 1");
    m.step(1);
    assert_eq!(m.title(), ("November".to_string(), "2026".to_string()), "Nov 2 to 8 is one month");
    m.pick_day(d(2026, 12, 30));
    assert_eq!(m.title(), ("Dec – Jan".to_string(), "2026 – 2027".to_string()));
}

#[test]
fn the_day_title_names_the_day() {
    let mut m = model_on(2026, 10, 28);
    m.set_view(View::Day);
    assert_eq!(m.title(), ("Wed 28 Oct".to_string(), "2026".to_string()));
    m.step(4);
    assert_eq!(m.title(), ("Sun 1 Nov".to_string(), "2026".to_string()));
}

#[test]
fn shown_days_are_one_day_or_the_week_from_the_week_start() {
    let mut m = model_on(2026, 10, 28);
    m.set_view(View::Day);
    let r = m.shown_days();
    assert_eq!((r.start, r.len()), (d(2026, 10, 28), 1));
    m.set_view(View::Week);
    let r = m.shown_days();
    assert_eq!((r.start, r.len()), (d(2026, 10, 26), 7), "weeks start on Monday by default");
    m.store.settings.week_start = chrono::Weekday::Sun;
    assert_eq!(m.shown_days().start, d(2026, 10, 25));
}
