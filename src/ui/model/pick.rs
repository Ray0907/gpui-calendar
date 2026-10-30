//! The mini-calendar month picker: where it starts, what picking a month does, and how the keys move through the grid.

use chrono::{Datelike, Duration, NaiveDate};

use super::{Model, add_months, month_start};

/// The years the picker offers; chrono's own range is far wider, but a calendar a few thousand years out is not a use.
pub const YEARS: std::ops::RangeInclusive<i32> = 1..=9999;

/// `(year, month 1..=12)` after `key` on the 3 by 4 grid: left and right by a month, up and down by a row, across the year edges;
/// Home and End to the year's first and last month. Past the first or last year the cursor stays. `None` for any other key.
pub fn month_cursor((year, month): (i32, u32), key: &str) -> Option<(i32, u32)> {
    let step = match key {
        "left" => -1,
        "right" => 1,
        "up" => -3,
        "down" => 3,
        "home" => return Some((year, 1)),
        "end" => return Some((year, 12)),
        _ => return None,
    };
    let at = year * 12 + month as i32 - 1 + step;
    let to = (at.div_euclid(12), at.rem_euclid(12) as u32 + 1);
    Some(if YEARS.contains(&to.0) { to } else { (year, month) })
}

impl Model {
    /// The month the mini calendar shows.
    pub fn mini_ym(&self) -> (i32, u32) {
        (self.mini.year(), self.mini.month())
    }

    /// Show `(year, month)` in the mini calendar and the view, as picking that day would: on the wanted day of the month, or its last day if it is shorter.
    pub fn pick_month(&mut self, year: i32, month: u32) {
        let first = NaiveDate::from_ymd_opt(year.clamp(*YEARS.start(), *YEARS.end()), month, 1).expect("month 1..=12");
        let last = month_start(add_months(first, 1)) - Duration::days(1);
        let (want, day) = (self.want_day, first.with_day(self.want_day.min(last.day())).expect("day within the month"));
        self.back_to_calendar();
        self.focus_anchor(day);
        self.want_day = want;
    }
}
