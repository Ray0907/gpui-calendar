use chrono::{Datelike, NaiveDate, Weekday};

use crate::domain::event::Event;
use crate::domain::year::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn at(m: u32, day: u32, h: u32) -> chrono::NaiveDateTime {
    d(2026, m, day).and_hms_opt(h, 0, 0).unwrap()
}

#[test]
fn a_year_has_twelve_grids_of_whole_weeks() {
    let g = year_grids(2026, Weekday::Mon);
    assert_eq!(g.len(), 12);
    assert_eq!(g.iter().map(|m| m.month).collect::<Vec<_>>(), (1..=12).collect::<Vec<_>>());
    for m in &g {
        assert_eq!(m.days.len() % 7, 0, "month {} is whole weeks", m.month);
        assert_eq!(m.days[0].weekday(), Weekday::Mon, "starts on the week start");
        assert!(m.days.contains(&d(2026, m.month, 1)));
    }
    // October 2026 starts on a Thursday: Sep 28 to Nov 1 is five weeks, and a sixth of Nov 2 to 8 pads it.
    assert_eq!(g[9].days.len(), 42);
    assert_eq!(g[9].days[0], d(2026, 9, 28));
    assert_eq!(g[9].days[41], d(2026, 11, 8));
    assert!(g[9].in_month(d(2026, 10, 31)) && !g[9].in_month(d(2026, 11, 1)));
}

#[test]
fn every_month_is_padded_to_six_week_rows_with_next_month_days() {
    for ws in [Weekday::Mon, Weekday::Sun] {
        for year in [2026, 2027, 2028] {
            for m in year_grids(year, ws) {
                assert_eq!((m.days.len(), m.rows().count()), (42, 6), "{year}-{} ({ws:?})", m.month);
                assert!(m.days.windows(2).all(|w| w[1] == w[0].succ_opt().unwrap()), "consecutive days");
                let (first, last) = (m.days.iter().position(|x| m.in_month(*x)).unwrap(), m.days.iter().rposition(|x| m.in_month(*x)).unwrap());
                assert!(first < 7, "the month starts in the first row");
                assert!(m.days[..first].iter().all(|x| x.month() != m.month) && m.days[last + 1..].iter().all(|x| x.month() != m.month));
            }
        }
    }
    // February 2027 is exactly four weeks starting Monday the 1st: two full rows of March pad it.
    let feb = &year_grids(2027, Weekday::Mon)[1];
    assert_eq!((feb.days[0], feb.days[41]), (d(2027, 2, 1), d(2027, 3, 14)));
}

#[test]
fn week_start_moves_the_first_column() {
    let sun = year_grids(2026, Weekday::Sun);
    assert_eq!(sun[9].days[0], d(2026, 9, 27));
}

#[test]
fn counts_cover_every_day_of_a_multi_day_event_and_only_the_shown_year() {
    let trip = Event::all_day(1, "Trip", 0, d(2026, 12, 30), d(2027, 1, 2));
    let late = Event::timed(2, "Late", 1, at(10, 30, 22), at(10, 31, 1));
    let marks = day_marks(&[&trip, &late], 2026);
    assert_eq!(marks[&d(2026, 12, 30)].count, 1);
    assert_eq!(marks[&d(2026, 12, 31)].count, 1);
    assert!(!marks.contains_key(&d(2027, 1, 1)), "other years are not counted");
    assert_eq!(marks[&d(2026, 10, 30)].count, 1);
    assert_eq!(marks[&d(2026, 10, 31)].count, 1, "a timed event past midnight marks the second day");
    assert!(!marks.contains_key(&d(2026, 11, 1)), "ends at 01:00, nothing on Nov 1 beyond that day");
}

#[test]
fn dots_are_one_per_calendar_in_calendar_order_and_the_count_stays_exact() {
    let evs = [
        Event::timed(1, "A", 2, at(10, 14, 9), at(10, 14, 10)),
        Event::timed(2, "B", 0, at(10, 14, 11), at(10, 14, 12)),
        Event::timed(3, "C", 0, at(10, 14, 13), at(10, 14, 14)),
        Event::timed(4, "D", 1, at(10, 14, 15), at(10, 14, 16)),
        Event::timed(5, "E", 3, at(10, 14, 17), at(10, 14, 18)),
    ];
    let refs: Vec<&Event> = evs.iter().collect();
    let m = &day_marks(&refs, 2026)[&d(2026, 10, 14)];
    assert_eq!(m.count, 5);
    assert_eq!(m.dots(), (&[0, 1, 2][..], 1), "three dots in calendar order, and the fourth calendar counted as hidden");
    let three = DayMarks { count: 3, calendars: vec![0, 1, 3] };
    assert_eq!(three.dots(), (&[0, 1, 3][..], 0), "three calendars never show a marker, whatever their theme");
    let two = DayMarks { count: 2, calendars: vec![0, 1] };
    assert_eq!(two.dots(), (&[0, 1][..], 0), "no marker when every calendar has a dot");
}

#[test]
fn a_multi_day_event_clamps_at_both_year_edges_and_its_end_is_exclusive() {
    let wrap = Event::all_day(1, "Wrap", 0, d(2025, 12, 30), d(2027, 1, 3));
    let marks = day_marks(&[&wrap], 2026);
    assert_eq!(marks.len(), 365, "every day of 2026 and no other");
    assert_eq!(marks[&d(2026, 1, 1)].count, 1);
    assert_eq!(marks[&d(2026, 12, 31)].count, 1);
    // A timed event ending exactly at midnight does not touch the next day.
    let to_midnight = Event::timed(2, "Late", 1, at(3, 4, 22), d(2026, 3, 5).and_hms_opt(0, 0, 0).unwrap());
    let m = day_marks(&[&to_midnight], 2026);
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), vec![d(2026, 3, 4)], "ends at 00:00 of the 5th: only the 4th");
    let span = Event::timed(3, "Span", 2, at(6, 1, 20), at(6, 3, 8));
    let m = day_marks(&[&span], 2026);
    assert_eq!((m.len(), m[&d(2026, 6, 2)].count), (3, 1));
    assert!(m.contains_key(&d(2026, 6, 1)) && m.contains_key(&d(2026, 6, 3)));
}

#[test]
fn no_events_no_marks() {
    assert!(day_marks(&[], 2026).is_empty());
}

#[test]
fn day_names_carry_the_date_and_the_count() {
    let m = DayMarks { count: 2, calendars: vec![0, 1] };
    assert_eq!(day_name(d(2026, 10, 28), Some(&m)), "Wednesday, October 28, 2026, 2 events");
    let one = DayMarks { count: 1, calendars: vec![0] };
    assert_eq!(day_name(d(2026, 10, 28), Some(&one)), "Wednesday, October 28, 2026, 1 event");
    assert_eq!(day_name(d(2026, 10, 29), None), "Thursday, October 29, 2026, no events");
}

#[test]
fn columns_follow_the_width_with_the_gaps_counted() {
    assert_eq!(columns_for(1076.), 4, "the 1440 window with its sidebar and the 24 padding");
    assert_eq!((columns_for(952.), columns_for(951.)), (4, 3), "four months of 220 with three 24 gaps are 952");
    assert_eq!(columns_for(2000.), 4, "never more than four");
    assert_eq!(columns_for(800.), 3, "the 960 window with a rail");
    // Three months of 220 with two 24 gaps are 708; two are 464; one is 220.
    assert_eq!((columns_for(708.), columns_for(707.)), (3, 2));
    assert_eq!((columns_for(464.), columns_for(463.)), (2, 1));
    assert_eq!((columns_for(220.), columns_for(0.)), (1, 1));
    assert_eq!(columns_for(660.), 2, "660 cannot hold three 220 months and the gaps");
    assert_eq!(columns_for(440.), 1, "440 is two months without the gap");
}

#[test]
fn the_mock_v8_year_metrics() {
    assert_eq!((MONTH_GAP, ROW_GAP, DOT_GAP, GRID_ROWS), (24., 16., 2., 6));
    assert!(year_grids(2026, Weekday::Mon).iter().all(|g| g.rows().count() == GRID_ROWS));
}

#[test]
fn months_sit_row_by_row() {
    assert_eq!((month_cell(1, 3), month_cell(3, 3), month_cell(4, 3), month_cell(12, 3)), ((0, 0), (0, 2), (1, 0), (3, 2)));
    assert_eq!(month_cell(12, 2), (5, 1));
    assert_eq!(month_at(1, 0, 3), Some(4));
    assert_eq!(month_at(3, 2, 3), Some(12));
    assert_eq!(month_at(4, 0, 3), None, "past December");
    assert_eq!(month_at(0, 3, 3), None, "past the last column");
    for cols in 1..=4 {
        for m in 1..=12 {
            let (r, c) = month_cell(m, cols);
            assert_eq!(month_at(r, c, cols), Some(m));
        }
    }
}

const MON: Weekday = Weekday::Mon;

fn step(cur: Cursor, dir: Dir) -> Cursor {
    step_cursor(cur, dir, 2026, MON)
}

fn day(m: u32, dd: u32) -> Cursor {
    Cursor::Day(d(2026, m, dd))
}

#[test]
fn arrows_walk_days_and_cross_months_but_stay_in_the_year() {
    assert_eq!(step(day(10, 28), Dir::Right), day(10, 29));
    assert_eq!(step(day(10, 31), Dir::Right), day(11, 1), "over the month edge");
    assert_eq!(step(day(11, 1), Dir::Left), day(10, 31));
    assert_eq!(step(day(10, 28), Dir::Down), day(11, 4));
    assert_eq!(step(day(10, 28), Dir::Up), day(10, 21));
    assert_eq!(step(day(12, 31), Dir::Right), day(12, 31), "no step out of the year");
    assert_eq!(step(day(12, 28), Dir::Down), day(12, 31), "a long step clamps to the last day");
}

#[test]
fn up_from_the_first_week_row_reaches_the_month_title_and_down_comes_back() {
    // Oct 2026 starts on Thursday: Oct 1 to 4 are the first Monday-first row.
    assert_eq!(step(day(10, 1), Dir::Up), Cursor::Title(2026, 10));
    assert_eq!(step(day(10, 4), Dir::Up), Cursor::Title(2026, 10));
    assert_eq!(step(day(10, 5), Dir::Up), day(9, 28), "the second row steps a week back, across the month edge");
    assert_eq!(step(day(10, 12), Dir::Up), day(10, 5));
}

#[test]
fn titles_move_sideways_by_month_and_down_into_the_month() {
    assert_eq!(step(Cursor::Title(2026, 10), Dir::Right), Cursor::Title(2026, 11));
    assert_eq!(step(Cursor::Title(2026, 12), Dir::Right), Cursor::Title(2026, 12));
    assert_eq!(step(Cursor::Title(2026, 1), Dir::Left), Cursor::Title(2026, 1));
    assert_eq!(step(Cursor::Title(2026, 10), Dir::Down), day(10, 1));
    assert_eq!(step(Cursor::Title(2026, 10), Dir::Up), Cursor::Title(2026, 10));
    assert_eq!((step(Cursor::Title(2026, 5), Dir::Home), step(Cursor::Title(2026, 5), Dir::End)), (Cursor::Title(2026, 1), Cursor::Title(2026, 12)));
    assert_eq!((step(Cursor::Title(2026, 5), Dir::PageUp), step(Cursor::Title(2026, 5), Dir::PageDown)), (Cursor::Title(2026, 4), Cursor::Title(2026, 6)));
}

#[test]
fn home_end_and_pages_follow_the_week_start_and_clamp() {
    // Wed Oct 28: the Monday-first week is Oct 26 to Nov 1.
    assert_eq!(step(day(10, 28), Dir::Home), day(10, 26));
    assert_eq!(step(day(10, 28), Dir::End), day(10, 31), "End stops on the last day of the month, not the Nov 1 filler");
    assert_eq!(step(day(10, 1), Dir::Home), day(10, 1), "Home stops on the first day of the month, not the Sep 28 filler");
    assert_eq!(step(day(11, 3), Dir::Home), day(11, 2));
    assert_eq!(step(day(11, 1), Dir::End), day(11, 1), "Nov 1 is the only November day of its row");
    assert_eq!(step_cursor(day(10, 28), Dir::Home, 2026, Weekday::Sun), day(10, 25));
    assert_eq!(step_cursor(day(2, 3), Dir::End, 2026, Weekday::Sun), day(2, 7));
    assert_eq!(step(day(1, 1), Dir::Home), day(1, 1), "the week of Jan 1 starts in 2025: clamped");
    assert_eq!(step(day(12, 31), Dir::End), day(12, 31));
    assert_eq!(step(day(10, 31), Dir::PageDown), day(11, 30), "the 31st lands on the last day of a shorter month");
    assert_eq!(step(day(3, 31), Dir::PageUp), day(2, 28));
    assert_eq!(step(day(1, 15), Dir::PageUp), day(1, 15), "no page out of the year");
    assert_eq!(step(day(12, 15), Dir::PageDown), day(12, 15));
}

#[test]
fn the_cursor_starts_on_the_anchor_or_inside_the_year() {
    assert_eq!(cursor_in(Some(day(5, 5)), d(2026, 10, 28), 2026), day(5, 5));
    assert_eq!(cursor_in(Some(Cursor::Day(d(2025, 5, 5))), d(2026, 10, 28), 2026), day(10, 28), "a stale one is dropped");
    assert_eq!(cursor_in(Some(Cursor::Title(2026, 3)), d(2026, 10, 28), 2026), Cursor::Title(2026, 3));
    assert_eq!(cursor_in(Some(Cursor::Title(2025, 3)), d(2026, 10, 28), 2026), day(10, 28), "a title of another year is dropped too");
    assert_eq!(cursor_in(None, d(2026, 10, 28), 2026), day(10, 28));
    assert_eq!(cursor_in(None, d(2025, 10, 28), 2026), day(1, 1), "an anchor of another year");
}

#[test]
fn day_height_never_squashes_the_date_box() {
    assert_eq!(DATE_BOX, 22., "guide: date box 22px");
    assert_eq!(day_height(100., 24), 27., "date box and the 5px dot row");
    assert!(day_height(0., 24) >= DATE_BOX + DOT_PX);
    assert_eq!(day_height(696., 24), 29.);
    assert_eq!(day_height(1000., 20), 30.);
    assert_eq!(day_height(500., 0), 30.);
}

#[test]
fn the_marker_reads_plus_n_and_caps_at_nine_plus() {
    assert_eq!((more_label(1), more_label(9), more_label(10), more_label(40)), ("+1".to_string(), "+9".to_string(), "9+".to_string(), "9+".to_string()));
}

#[test]
fn a_week_with_a_marker_is_tall_enough_for_its_own_line_box() {
    assert_eq!(row_height(27., false), 27.);
    assert_eq!(row_height(27., true), DATE_BOX + MORE_H);
    assert_eq!(row_height(DAY_MAX_H, true), DAY_MAX_H);
}

#[test]
fn marked_weeks_flags_the_week_rows_that_hold_a_plus_marker() {
    let crowd: Vec<Event> = (0..5).map(|c| Event::timed(c as u64 + 1, "E", c, at(10, 14, 9), at(10, 14, 10))).collect();
    let marks = day_marks(&crowd.iter().collect::<Vec<_>>(), 2026);
    let oct = year_grids(2026, Weekday::Mon).remove(9);
    assert_eq!(oct.marked_weeks(&marks), vec![false, false, true, false, false, false], "Oct 14 is in the third row");
    assert_eq!(year_grids(2026, Weekday::Mon)[0].marked_weeks(&marks).iter().filter(|m| **m).count(), 0);
}
