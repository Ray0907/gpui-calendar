//! What the Year view derives from the year, the first weekday and the visible events: the month grids,
//! the day marks and the day names. Rebuilt only when one of them changes, not on every frame.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use chrono::{NaiveDate, Weekday};
use gpui_kit::SharedString;

use crate::domain::dates::year_range;
use crate::domain::event::Event;
use crate::domain::year::{DayMarks, MonthGrid, day_marks, day_name, year_grids};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Key(i32, Weekday, u64);

impl Key {
    pub fn new(year: i32, ws: Weekday, events: &[&Event]) -> Self {
        let mut h = DefaultHasher::new();
        for e in events {
            (e.id, e.calendar, e.start, e.end, e.all_day).hash(&mut h);
        }
        Key(year, ws, h.finish())
    }
}

pub(super) struct Cache {
    pub key: Key,
    pub grids: Vec<MonthGrid>,
    pub marks: HashMap<NaiveDate, DayMarks>,
    pub names: HashMap<NaiveDate, SharedString>,
}

impl Cache {
    pub fn build(key: Key, events: &[&Event]) -> Self {
        let Key(year, ws, _) = key;
        let marks = day_marks(events, year);
        let names = year_range(year).days().map(|d| (d, SharedString::from(day_name(d, marks.get(&d))))).collect();
        Self { key, grids: year_grids(year, ws), marks, names }
    }
}
