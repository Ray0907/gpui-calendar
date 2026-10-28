use chrono::{NaiveDate, Weekday};

use crate::domain::month::*;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// 21 weeks, Aug 17, 2026 to Jan 3, 2027; the October grid starts at week 6.
fn weeks() -> Weeks {
    Weeks::around(d(2026, 10, 28), Weekday::Mon, 10)
}

const LAID_OUT: Option<usize> = Some(18); // 300px viewport of 100px rows

#[test]
fn scrolls_to_the_anchor_first_then_waits_for_the_list_to_arrive() {
    let w = weeks();
    let mut s = SyncState::default();
    let oct = w.first_week_of((2026, 10));
    assert_eq!(oct, 6);
    assert_eq!(s.step(d(2026, 10, 28), 0, LAID_OUT, &w), Plan::Scroll(oct));
    assert_eq!(s.step(d(2026, 10, 28), oct, LAID_OUT, &w), Plan::Idle, "arrived");
    assert_eq!(s.step(d(2026, 10, 28), oct, LAID_OUT, &w), Plan::Idle);
}

#[test]
fn lets_the_viewer_scroll_the_title_and_navigation_scroll_the_list() {
    let w = weeks();
    let mut s = SyncState::default();
    s.step(d(2026, 10, 28), 0, LAID_OUT, &w);
    s.step(d(2026, 10, 28), 6, LAID_OUT, &w);
    // The viewer scrolls one week past October's end: November leads.
    assert_eq!(s.step(d(2026, 10, 28), 11, LAID_OUT, &w), Plan::Follow((2026, 11)));
    s.followed(d(2026, 11, 28));
    assert_eq!(s.step(d(2026, 11, 28), 11, LAID_OUT, &w), Plan::Idle, "the anchor caught up; no scroll back");
    // Next-month button: the anchor moved, the list has not.
    assert_eq!(s.step(d(2026, 12, 28), 11, LAID_OUT, &w), Plan::Scroll(w.first_week_of((2026, 12))));
}

#[test]
fn a_target_in_the_month_already_shown_still_scrolls_to_its_day() {
    let w = weeks();
    let mut s = SyncState::default();
    s.step(d(2026, 10, 6), 0, LAID_OUT, &w);
    assert_eq!(s.step(d(2026, 10, 6), 6, LAID_OUT, &w), Plan::Idle);
    // Mini month: Oct 28. Its week (10) would be named November at the top, so the list stops one above.
    assert_eq!(s.step(d(2026, 10, 28), 6, LAID_OUT, &w), Plan::Scroll(9));
    assert_eq!(s.step(d(2026, 10, 28), 9, LAID_OUT, &w), Plan::Idle, "arrived");
    assert_eq!(s.step(d(2026, 10, 28), 9, LAID_OUT, &w), Plan::Idle, "and the title still says October");
    // Back to a day in the first week: Today.
    assert_eq!(s.step(d(2026, 10, 2), 9, LAID_OUT, &w), Plan::Scroll(6));
}

#[test]
fn a_same_month_target_after_following_the_viewer_is_not_dropped_either() {
    let w = weeks();
    let mut s = SyncState::default();
    s.step(d(2026, 10, 28), 0, LAID_OUT, &w);
    s.step(d(2026, 10, 28), 6, LAID_OUT, &w);
    assert_eq!(s.step(d(2026, 10, 28), 14, LAID_OUT, &w), Plan::Follow(w.month_of_week(14)));
    s.followed(d(2026, 11, 28));
    assert_eq!(s.step(d(2026, 11, 28), 14, LAID_OUT, &w), Plan::Idle);
    assert_eq!(s.step(d(2026, 11, 3), 14, LAID_OUT, &w), Plan::Scroll(11), "Nov 3 is in week 11");
}

#[test]
fn never_drops_a_target_before_the_list_is_laid_out() {
    let w = weeks();
    let mut s = SyncState::default();
    // A slow first layout: the offset stays at the top for far more than five renders.
    for i in 0..50 {
        assert_eq!(s.step(d(2026, 10, 28), 0, None, &w), Plan::Scroll(6), "render {i}");
    }
    assert_eq!(s.step(d(2026, 10, 28), 6, LAID_OUT, &w), Plan::Idle, "then it lands");
}

#[test]
fn gives_the_lead_back_to_a_viewer_who_scrolled_after_the_ask() {
    let w = weeks();
    let mut s = SyncState::default();
    assert_eq!(s.step(d(2026, 10, 28), 3, LAID_OUT, &w), Plan::Scroll(6));
    assert_eq!(s.step(d(2026, 10, 28), 3, LAID_OUT, &w), Plan::Scroll(6), "one more ask for a stale offset");
    assert_eq!(s.step(d(2026, 10, 28), 3, LAID_OUT, &w), Plan::Follow(w.month_of_week(3)), "the list is where the viewer put it");
}

#[test]
fn a_target_past_either_end_lands_at_the_nearest_week_and_keeps_the_anchor() {
    let w = weeks();
    assert_eq!(w.last_top(300., 100.), 18, "the last week that can sit at the top");
    let mut s = SyncState::default();
    // June 2027 is past the run (it ends in the week of Dec 28, 2026).
    let far = d(2027, 6, 28);
    assert_eq!(s.step(far, 0, LAID_OUT, &w), Plan::Scroll(18));
    assert_eq!(s.step(far, 18, LAID_OUT, &w), Plan::Idle, "landed at the nearest valid week");
    assert_eq!(s.step(far, 18, LAID_OUT, &w), Plan::Idle, "the anchor is not rewritten to the month there");
    assert_eq!(s.step(far, 12, LAID_OUT, &w), Plan::Follow(w.month_of_week(12)), "until the viewer scrolls away");
    let mut s = SyncState::default();
    let early = d(2020, 1, 15);
    assert_eq!(s.step(early, 9, LAID_OUT, &w), Plan::Scroll(0));
    assert_eq!(s.step(early, 0, LAID_OUT, &w), Plan::Idle);
    assert_eq!(s.step(early, 0, LAID_OUT, &w), Plan::Idle);
}

#[test]
fn a_month_the_list_cannot_scroll_to_is_clamped_without_losing_the_anchor() {
    // January's first week (19) is below the last top week (18): the list rests on December.
    let w = weeks();
    let mut s = SyncState::default();
    assert_eq!(s.step(d(2027, 1, 5), 0, LAID_OUT, &w), Plan::Scroll(18));
    assert_eq!(s.step(d(2027, 1, 5), 18, LAID_OUT, &w), Plan::Idle);
    assert_eq!(s.step(d(2027, 1, 5), 18, LAID_OUT, &w), Plan::Idle);
}
