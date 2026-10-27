use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike, Weekday};
use serde::{Deserialize, Serialize};

/// Half-open date range `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn len(&self) -> usize {
        (self.end - self.start).num_days().max(0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, d: NaiveDate) -> bool {
        self.start <= d && d < self.end
    }

    pub fn days(&self) -> impl Iterator<Item = NaiveDate> {
        let start = self.start;
        (0..self.len() as i64).map(move |i| start + Duration::days(i))
    }
}

pub fn week_start_of(d: NaiveDate, ws: Weekday) -> NaiveDate {
    let back = (d.weekday().num_days_from_monday() + 7 - ws.num_days_from_monday()) % 7;
    d - Duration::days(back as i64)
}

pub fn week_range(d: NaiveDate, ws: Weekday) -> DateRange {
    let start = week_start_of(d, ws);
    DateRange { start, end: start + Duration::days(7) }
}

/// First of the month up to the first of the next.
///
/// # Panics
/// If `month` is not in `1..=12` or the date is outside chrono's range.
pub fn month_range(year: i32, month: u32) -> DateRange {
    let start = NaiveDate::from_ymd_opt(year, month, 1).expect("valid year/month");
    DateRange { start, end: add_months(start, 1) }
}

/// Whole weeks covering the month (what a month grid shows).
pub fn month_grid_range(year: i32, month: u32, ws: Weekday) -> DateRange {
    let m = month_range(year, month);
    let start = week_start_of(m.start, ws);
    let last = m.end - Duration::days(1);
    DateRange { start, end: week_range(last, ws).end }
}

/// # Panics
/// If `year` (or `year + 1`) is outside chrono's range.
pub fn year_range(year: i32) -> DateRange {
    DateRange {
        start: NaiveDate::from_ymd_opt(year, 1, 1).expect("valid year"),
        end: NaiveDate::from_ymd_opt(year + 1, 1, 1).expect("valid year"),
    }
}

/// Shift by whole months, clamping the day to the target month's length.
pub fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    let m0 = d.year() * 12 + d.month0() as i32 + n;
    let (y, m) = (m0.div_euclid(12), m0.rem_euclid(12) as u32 + 1);
    (1..=d.day())
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(y, m, day))
        .expect("day 1 always exists")
}

pub fn format_time(t: NaiveTime, hour24: bool) -> String {
    if hour24 {
        return format!("{:02}:{:02}", t.hour(), t.minute());
    }
    let (pm, h) = t.hour12();
    format!("{}:{:02} {}", h, t.minute(), if pm { "PM" } else { "AM" })
}

/// Hour-axis label.
pub fn format_hour(hour: u32, hour24: bool) -> String {
    if hour24 {
        return format!("{hour:02}:00");
    }
    let h = if hour.is_multiple_of(12) { 12 } else { hour % 12 };
    format!("{h} {}", if hour % 24 >= 12 { "PM" } else { "AM" })
}
