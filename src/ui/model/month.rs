//! Month view state on the model: which events show, following the scroll, and editing by drag.

use std::collections::HashSet;

use chrono::{Datelike, NaiveDate};

use super::{Model, month_start};
use crate::domain::drag::Preview;
use crate::domain::event::{CalendarId, Event, visible};
use crate::domain::month::YearMonth;

pub const NEW_EVENT_TITLE: &str = "New event";

impl Model {
    pub fn hidden_calendars(&self) -> HashSet<CalendarId> {
        self.store.calendars.iter().filter(|c| !c.visible).map(|c| c.id).collect()
    }

    /// Events of calendars that are switched on.
    pub fn visible_events(&self) -> Vec<&Event> {
        visible(&self.store.events, &self.hidden_calendars())
    }

    pub fn calendar_name(&self, id: CalendarId) -> &str {
        self.store.calendars.iter().find(|c| c.id == id).map_or("", |c| c.name.as_str())
    }

    /// Where a new event goes: the first calendar that is switched on. `None` if all are off:
    /// a new event there would be invisible, so nothing is created.
    pub fn new_event_calendar(&self) -> Option<CalendarId> {
        self.store.calendars.iter().find(|c| c.visible).map(|c| c.id)
    }

    /// The viewer scrolled into `(year, month)`: the header and mini month show it, on the day
    /// they last chose or the month's last day if it is shorter. No scroll is requested.
    pub fn follow_month(&mut self, (year, month): YearMonth) {
        let first = NaiveDate::from_ymd_opt(year, month, 1).expect("valid year/month");
        let days = (month_start(crate::domain::dates::add_months(first, 1)) - first).num_days() as u32;
        self.anchor = first.with_day(self.want_day.min(days)).expect("day within the month");
        self.base = self.anchor;
        self.steps = 0;
        self.mini = month_start(self.anchor);
    }

    pub fn anchor_month(&self) -> YearMonth {
        (self.anchor.year(), self.anchor.month())
    }

    /// A new all-day event from a drag across days; `None` if no calendar is switched on.
    pub fn create_from_drag(&mut self, p: Preview) -> Option<u64> {
        let calendar = self.new_event_calendar()?;
        let id = self.store.events.iter().map(|e| e.id).max().map_or(1, |m| m + 1);
        self.store.events.push(Event { id, title: NEW_EVENT_TITLE.into(), calendar, start: p.start, end: p.end, all_day: p.all_day });
        self.persist();
        Some(id)
    }

    /// Move an existing event as the drag previewed. `false` if it is gone.
    pub fn move_from_drag(&mut self, id: u64, p: Preview) -> bool {
        let Some(e) = self.store.events.iter_mut().find(|e| e.id == id) else { return false };
        (e.start, e.end, e.all_day) = (p.start, p.end, p.all_day);
        self.persist();
        true
    }
}
