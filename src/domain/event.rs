use std::collections::HashSet;

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use super::dates::DateRange;
use super::drag::Preview;
use super::layout::SNAP_MINUTES;

pub type CalendarId = u8;

/// Local floating-time event. `end` is exclusive.
/// All-day events: `start` is 00:00 of the first day, `end` is 00:00 of the day after the last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub title: String,
    pub calendar: CalendarId,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub all_day: bool,
    /// Free text; files saved before notes existed load with it empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

pub const MINUTES_PER_DAY: u32 = 1440;

impl Event {
    /// A timed event is at least 15 minutes long.
    pub fn timed(id: u64, title: &str, calendar: CalendarId, start: NaiveDateTime, end: NaiveDateTime) -> Self {
        let end = end.max(start + Duration::minutes(SNAP_MINUTES as i64));
        Self { id, title: title.into(), calendar, start, end, all_day: false, notes: String::new() }
    }

    /// Covers `first..=last` (a `last` before `first` means a single day).
    pub fn all_day(id: u64, title: &str, calendar: CalendarId, first: NaiveDate, last: NaiveDate) -> Self {
        let last = last.max(first);
        Self {
            id,
            title: title.into(),
            calendar,
            start: first.and_time(NaiveTime::MIN),
            end: (last + Duration::days(1)).and_time(NaiveTime::MIN),
            all_day: true,
            notes: String::new(),
        }
    }

    /// A new event as a create drag previewed it.
    pub fn from_preview(id: u64, title: &str, calendar: CalendarId, p: &Preview) -> Self {
        if p.all_day {
            Self::all_day(id, title, calendar, p.start.date(), (p.end - Duration::days(1)).date())
        } else {
            Self::timed(id, title, calendar, p.start, p.end)
        }
    }

    /// Dates the event touches, `[first, after_last)`.
    pub fn days(&self) -> DateRange {
        let start = self.start.date();
        let last = self.end.max(self.start + Duration::minutes(SNAP_MINUTES as i64)); // zero-length via serde
        let end = if last.time() == NaiveTime::MIN { last.date() } else { last.date() + Duration::days(1) };
        DateRange { start, end: end.max(start + Duration::days(1)) }
    }

    pub fn occupies(&self, day: NaiveDate) -> bool {
        self.days().contains(day)
    }

    /// Minutes-of-day `[start, end)` this event covers on `day`; None if all-day or not on `day`.
    pub fn segment_on(&self, day: NaiveDate) -> Option<(u32, u32)> {
        if self.all_day {
            return None;
        }
        let day_start = day.and_time(NaiveTime::MIN);
        let lo = self.start.max(day_start);
        let hi = self.end.max(self.start + Duration::minutes(SNAP_MINUTES as i64)).min(day_start + Duration::days(1));
        (lo < hi).then(|| ((lo - day_start).num_minutes() as u32, (hi - day_start).num_minutes() as u32))
    }

    /// Routing rule: only `all_day` events live in the all-day row. A timed event, even one
    /// crossing midnight, is drawn on the time grid via `segment_on` for each day it touches
    /// (month view draws it as a chip/bar per `days()`); it is never in both places.
    pub fn in_all_day_row(&self) -> bool {
        self.all_day
    }
}

/// Events touching any date in `r`, in input order.
pub fn events_in_range(events: &[Event], r: DateRange) -> Vec<&Event> {
    if r.is_empty() {
        return Vec::new();
    }
    events.iter().filter(|e| e.days().start < r.end && e.days().end > r.start).collect()
}

pub fn events_on_day(events: &[Event], day: NaiveDate) -> Vec<&Event> {
    events.iter().filter(|e| e.occupies(day)).collect()
}

/// Events whose calendar is not hidden.
pub fn visible<'a>(events: &'a [Event], hidden: &HashSet<CalendarId>) -> Vec<&'a Event> {
    events.iter().filter(|e| !hidden.contains(&e.calendar)).collect()
}
