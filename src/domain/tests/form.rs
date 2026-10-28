use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::domain::event::Event;
use crate::domain::form::*;

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}
fn dt(day: u32, h: u32, m: u32) -> NaiveDateTime {
    d(day).and_hms_opt(h, m, 0).unwrap()
}
fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).unwrap()
}
fn lunch() -> Event {
    Event::timed(7, "Team lunch", 1, dt(28, 12, 30), dt(28, 13, 30))
}
fn fields(r: Result<Event, Vec<FormError>>) -> Vec<Field> {
    r.unwrap_err().iter().map(|e| e.field).collect()
}

#[test]
fn parse_time_takes_24h_and_12h_text() {
    assert_eq!(parse_time("14:30", true), Some(t(14, 30)));
    assert_eq!(parse_time(" 9:05 ", true), Some(t(9, 5)));
    assert_eq!(parse_time("2:30 PM", true), Some(t(14, 30)));
    assert_eq!(parse_time("2:30pm", true), Some(t(14, 30)));
    assert_eq!(parse_time("12:00 AM", true), Some(t(0, 0)));
    assert_eq!(parse_time("12:15 PM", true), Some(t(12, 15)));
    for bad in ["", "24:00", "13:60", "0:30 PM", "13:00 PM", "9", "ab:cd", "9:5", "09:30:00"] {
        assert_eq!(parse_time(bad, true), None, "{bad:?}");
    }
}

#[test]
fn in_12h_mode_a_time_needs_am_or_pm() {
    assert_eq!(parse_time("2:30", false), None, "2:30 is not silently 02:30");
    assert_eq!(parse_time("14:30", false), None);
    assert_eq!(parse_time("2:30 pm", false), Some(t(14, 30)));
    assert_eq!(parse_time("2:30", true), Some(t(2, 30)));
    let mut x = Draft::for_event(&lunch(), false);
    x.start_time = "2:30".into();
    assert_eq!(fields(x.check(7, false)), [Field::StartTime]);
    assert_eq!(x.check(7, true).map(|e| e.start), Ok(dt(28, 2, 30)));
}

#[test]
fn parse_date_takes_iso_only() {
    assert_eq!(parse_date("2026-10-28"), Some(d(28)));
    assert_eq!(parse_date(" 2026-10-28 "), Some(d(28)));
    for bad in ["", "28/10/2026", "2026-13-01", "2026-02-30", "tomorrow"] {
        assert_eq!(parse_date(bad), None, "{bad:?}");
    }
}

#[test]
fn a_draft_shows_times_in_the_chosen_format() {
    let x = Draft::for_event(&lunch(), true);
    assert_eq!((x.start_date.as_str(), x.start_time.as_str(), x.end_time.as_str()), ("2026-10-28", "12:30", "13:30"));
    let x = Draft::for_event(&lunch(), false);
    assert_eq!((x.start_time.as_str(), x.end_time.as_str()), ("12:30 PM", "1:30 PM"));
    assert_eq!((x.id, x.title.as_str(), x.calendar, x.all_day), (Some(7), "Team lunch", 1, false));
}

#[test]
fn an_unchanged_draft_gives_the_event_back() {
    let e = lunch();
    assert_eq!(Draft::for_event(&e, true).check(7, true), Ok(e.clone()));
    assert_eq!(Draft::for_event(&e, false).check(7, false), Ok(e));
    let a = Event::all_day(3, "Conference", 0, d(3), d(5));
    let x = Draft::for_event(&a, true);
    assert_eq!((x.start_date.as_str(), x.end_date.as_str()), ("2026-10-03", "2026-10-05"));
    assert_eq!(x.check(3, true), Ok(a), "the form shows the last day, the event stores the day after");
}

#[test]
fn editing_title_and_time_changes_only_those() {
    let mut x = Draft::for_event(&lunch(), true);
    x.title = "  Planning  ".into();
    x.start_time = "13:00".into();
    x.end_time = "14:15".into();
    let e = x.check(7, true).unwrap();
    assert_eq!((e.title.as_str(), e.start, e.end, e.calendar, e.all_day), ("Planning", dt(28, 13, 0), dt(28, 14, 15), 1, false));
}

#[test]
fn an_empty_title_is_the_only_error_it_raises() {
    let mut x = Draft::for_event(&lunch(), true);
    x.title = "   ".into();
    let errs = x.check(7, true).unwrap_err();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].field, Field::Title);
    assert_eq!(errs[0].message, "Enter a title.");
}

#[test]
fn the_end_must_be_after_the_start() {
    let mut x = Draft::for_event(&lunch(), true);
    x.end_time = "12:30".into();
    assert_eq!(fields(x.check(7, true)), [Field::EndTime], "equal is not after");
    x.end_time = "11:00".into();
    assert_eq!(fields(x.check(7, true)), [Field::EndTime]);
    x.end_date = "2026-10-29".into();
    assert_eq!(x.check(7, true).unwrap().end, dt(29, 11, 0), "a later end date fixes it");
    x.end_date = "2026-10-27".into();
    assert_eq!(fields(x.check(7, true)), [Field::EndTime]);
}

#[test]
fn bad_text_names_the_field() {
    let mut x = Draft::for_event(&lunch(), true);
    (x.start_date, x.start_time, x.end_date, x.end_time) = ("x".into(), "99".into(), "y".into(), "".into());
    assert_eq!(fields(x.check(7, true)), [Field::StartDate, Field::StartTime, Field::EndDate, Field::EndTime]);
}

#[test]
fn errors_come_in_field_order_with_the_title_first() {
    let mut x = Draft::for_event(&lunch(), true);
    x.title.clear();
    x.start_time = "nope".into();
    assert_eq!(fields(x.check(7, true)), [Field::Title, Field::StartTime]);
}

#[test]
fn all_day_ignores_the_times() {
    let mut x = Draft::for_event(&lunch(), true);
    x.set_all_day(true, true);
    x.start_time = "garbage".into();
    let e = x.check(7, true).unwrap();
    assert!(e.all_day);
    assert_eq!((e.start, e.end), (dt(28, 0, 0), dt(29, 0, 0)));
}

/// A blank form for `day`, switched to all day.
fn all_day(day: NaiveDate) -> Draft {
    let mut x = Draft::new_at(day.and_hms_opt(9, 0, 0).unwrap(), 0, true);
    x.set_all_day(true, true);
    x
}

#[test]
fn all_day_end_before_start_is_an_error_on_the_end_date() {
    let mut x = all_day(d(10));
    x.title = "Trip".into();
    x.end_date = "2026-10-09".into();
    assert_eq!(fields(x.check(1, true)), [Field::EndDate]);
}

#[test]
fn turning_all_day_off_brings_back_working_times() {
    let mut x = all_day(d(10));
    x.title = "x".into();
    x.set_all_day(false, true);
    assert_eq!((x.start_time.as_str(), x.end_time.as_str()), ("09:00", "10:00"));
    assert_eq!(x.check(1, true).unwrap().end, dt(10, 10, 0));
    x.set_all_day(true, true);
    x.set_all_day(false, false);
    assert_eq!(x.start_time, "9:00 AM", "kept times follow a new format");
    assert_eq!(x.end_time, "10:00 AM");
}

#[test]
fn a_new_event_snaps_down_and_lasts_an_hour() {
    let x = Draft::new_at(dt(28, 14, 37), 2, true);
    assert_eq!((x.id, x.title.as_str(), x.calendar, x.all_day), (None, "", 2, false));
    assert_eq!((x.start_date.as_str(), x.start_time.as_str(), x.end_date.as_str(), x.end_time.as_str()), ("2026-10-28", "14:30", "2026-10-28", "15:30"));
    let x = Draft::new_at(dt(28, 23, 45), 0, true);
    assert_eq!((x.end_date.as_str(), x.end_time.as_str()), ("2026-10-29", "00:45"));
}

#[test]
fn the_add_button_picks_the_next_hour_today_and_nine_elsewhere() {
    assert_eq!(default_start(d(28), dt(28, 17, 35)), dt(28, 18, 0));
    assert_eq!(default_start(d(28), dt(28, 18, 0)), dt(28, 19, 0));
    assert_eq!(default_start(d(28), dt(28, 23, 10)), dt(28, 23, 0), "the last hour of the day stays on the day");
    assert_eq!(default_start(d(30), dt(28, 17, 35)), dt(30, 9, 0));
}

#[test]
fn ids_are_one_past_the_largest() {
    assert_eq!(next_id(&[]), 1);
    assert_eq!(next_id(&[lunch()]), 8);
}

#[test]
fn save_adds_or_replaces_and_reports_the_edit() {
    let mut ev = vec![lunch()];
    let mut moved = lunch();
    moved.title = "Brunch".into();
    let edit = save(&mut ev, moved.clone());
    assert_eq!((ev.len(), ev[0].title.as_str()), (1, "Brunch"));
    assert_eq!(edit, Edit::Change { before: lunch(), after: moved });
    let new = Event::timed(8, "New", 0, dt(29, 9, 0), dt(29, 10, 0));
    assert_eq!(save(&mut ev, new.clone()), Edit::Add(new));
    assert_eq!(ev.len(), 2);
}

#[test]
fn every_edit_reverts_and_reapplies_exactly() {
    let a = Event::timed(1, "a", 0, dt(26, 9, 0), dt(26, 10, 0));
    let b = lunch();
    let c = Event::timed(9, "c", 2, dt(30, 9, 0), dt(30, 10, 0));
    let base = vec![a, b.clone(), c];

    let mut ev = base.clone();
    let edit = remove(&mut ev, 7).unwrap();
    assert_eq!(edit, Edit::Remove { index: 1, event: b.clone() });
    assert_eq!(ev.len(), 2);
    edit.revert(&mut ev);
    assert_eq!(ev, base, "the event is back at its old place");
    edit.apply(&mut ev);
    assert_eq!(ev.len(), 2);

    let mut ev = base.clone();
    let mut changed = b.clone();
    changed.title = "z".into();
    let edit = save(&mut ev, changed);
    edit.revert(&mut ev);
    assert_eq!(ev, base);
    edit.apply(&mut ev);
    assert_eq!(ev[1].title, "z");

    let mut ev = base.clone();
    let edit = save(&mut ev, Event::timed(10, "n", 0, dt(31, 9, 0), dt(31, 10, 0)));
    edit.revert(&mut ev);
    assert_eq!(ev, base);
}

#[test]
fn removing_a_missing_event_is_nothing() {
    let mut ev = vec![lunch()];
    assert_eq!(remove(&mut ev, 99), None);
    assert_eq!(ev.len(), 1);
}

#[test]
fn a_calendar_change_is_kept_by_check() {
    let mut x = Draft::for_event(&lunch(), true);
    x.calendar = 2;
    assert_eq!(x.check(7, true).unwrap().calendar, 2);
}

#[test]
fn the_well_says_when_and_for_how_long() {
    let today = d(28);
    let mut x = Draft::for_event(&lunch(), true);
    assert_eq!(summary(&x, today, true), Some(("Today, 12:30 – 13:30".into(), "1 hour · Wednesday, Oct 28".into())));
    assert_eq!(summary(&Draft::for_event(&lunch(), false), today, false).unwrap().0, "Today, 12:30 PM – 1:30 PM");
    assert_eq!(summary(&x, d(27), true).unwrap().0, "Tomorrow, 12:30 – 13:30");
    assert_eq!(summary(&x, d(20), true).unwrap().0, "Wed, Oct 28, 12:30 – 13:30");
    x.end_time = "13:00".into();
    assert_eq!(summary(&x, today, true).unwrap().1, "30 min · Wednesday, Oct 28");
    x.end_date = "2026-10-29".into();
    x.end_time = "14:00".into();
    assert_eq!(summary(&x, today, true), Some(("Today, 12:30 – Tomorrow, 14:00".into(), "25 h 30 min · Wednesday, Oct 28".into())));
    x.end_time = "oops".into();
    assert_eq!(summary(&x, today, true), None);
}

#[test]
fn the_well_for_all_day_counts_days() {
    let mut x = all_day(d(28));
    x.title = "t".into();
    assert_eq!(summary(&x, d(28), true), Some(("Today".into(), "All day · 1 day · Wednesday, Oct 28".into())));
    x.end_date = "2026-10-30".into();
    assert_eq!(summary(&x, d(28), true), Some(("Today – Fri, Oct 30".into(), "All day · 3 days · Wednesday, Oct 28".into())));
}
