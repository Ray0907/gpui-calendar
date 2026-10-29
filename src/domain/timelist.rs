//! The rows of a time picker's list: every 15 minutes of the day, the end list from the start time on, and the duration shown after each end time.

use chrono::NaiveTime;

/// Rows in `start_rows`/`end_rows` are this many minutes apart.
pub const STEP_MIN: u32 = 15;

fn grid() -> impl Iterator<Item = NaiveTime> {
    (0..24 * 60 / STEP_MIN).filter_map(|i| NaiveTime::from_hms_opt(i * STEP_MIN / 60, i * STEP_MIN % 60, 0))
}

/// The grid plus `extra`, in order: a time that is not on a quarter hour still has its row.
fn with(extra: impl IntoIterator<Item = NaiveTime>, keep: impl Fn(&NaiveTime) -> bool) -> Vec<NaiveTime> {
    let mut rows: Vec<_> = grid().chain(extra).filter(|t| keep(t)).collect();
    rows.sort();
    rows.dedup();
    rows
}

pub fn start_rows(current: NaiveTime) -> Vec<NaiveTime> {
    with([current], |_| true)
}

/// On the start's own day the list begins at the start time and offers nothing earlier; on a later day it is the whole day.
pub fn end_rows(start: NaiveTime, current: NaiveTime, same_day: bool) -> Vec<NaiveTime> {
    if same_day { with([start, current], |&t| t >= start) } else { with([current], |_| true) }
}

/// `0 min`, `45 min`, `1 h`, `1 h 15 min`.
pub fn duration(minutes: i64) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}
