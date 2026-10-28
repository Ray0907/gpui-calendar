//! Year view, pure: the twelve month grids, per-day event marks, layout columns and arrow movement.

use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::dates::{DateRange, month_grid_range, month_range, week_start_of, year_range};
use super::event::{CalendarId, Event};

/// One mini month: always six week rows (the mock's), so out-of-month days pad the first row and the tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthGrid {
    pub month: u32,
    pub days: Vec<NaiveDate>,
}

impl MonthGrid {
    pub fn rows(&self) -> impl Iterator<Item = &[NaiveDate]> {
        self.days.chunks(7)
    }

    /// Per week row: whether one of its days of this month shows a "+n" marker.
    pub fn marked_weeks(&self, marks: &HashMap<NaiveDate, DayMarks>) -> Vec<bool> {
        self.rows().map(|row| row.iter().any(|d| self.in_month(*d) && marks.get(d).is_some_and(|m| m.dots().1 > 0))).collect()
    }

    pub fn in_month(&self, d: NaiveDate) -> bool {
        d.month() == self.month
    }
}

/// Week rows of every month grid, and its days.
pub const GRID_ROWS: usize = 6;
const GRID_DAYS: usize = GRID_ROWS * 7;

pub fn year_grids(year: i32, ws: Weekday) -> Vec<MonthGrid> {
    (1..=12).map(|month| MonthGrid { month, days: week_start_of(month_range(year, month).start, ws).iter_days().take(GRID_DAYS).collect() }).collect()
}

/// Dots a day shows at most; more calendars than this show a "+n" marker.
pub const MAX_DOTS: usize = 3;

/// The date box of a day (guide: 22px tall for every date) and the dot row under it.
pub const DATE_BOX: f32 = 22.;
/// Side of a dot (the mock's), which is also the height of the dot row, and the gap between dots.
pub const DOT_PX: f32 = 5.;
pub const DOT_GAP: f32 = 2.;
/// Line box of the "+n" marker: its 8px text needs more than the dot row.
pub const MORE_H: f32 = 8.;
/// Tallest a day grows (the mock's), and the most month columns.
pub const DAY_MAX_H: f32 = 30.;
pub const MAX_COLS: usize = 4;

/// The "+n" marker text: counts above nine read "9+", so the row never outgrows a narrow day.
pub fn more_label(hidden: usize) -> String {
    if hidden > 9 { "9+".to_string() } else { format!("+{hidden}") }
}

/// Height of a week row: `day_h`, or the least that holds the date box and a "+n" line box when a day has a marker.
pub fn row_height(day_h: f32, has_more: bool) -> f32 {
    if has_more { day_h.max(DATE_BOX + MORE_H) } else { day_h }
}

/// What one day holds: how many events touch it and which calendars they belong to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayMarks {
    pub count: usize,
    /// Distinct calendar ids, ascending.
    pub calendars: Vec<CalendarId>,
}

impl DayMarks {
    /// The calendars drawn as dots (shape, and color when the calendar has a theme), in calendar order,
    /// and how many calendars are left out: only those past `MAX_DOTS`, which show as "+n".
    pub fn dots(&self) -> (&[CalendarId], usize) {
        let shown = self.calendars.len().min(MAX_DOTS);
        (&self.calendars[..shown], self.calendars.len() - shown)
    }
}

/// Marks for the days of `year` that have events. `events` should already be the visible ones.
pub fn day_marks(events: &[&Event], year: i32) -> HashMap<NaiveDate, DayMarks> {
    let shown: DateRange = year_range(year);
    let mut marks: HashMap<NaiveDate, DayMarks> = HashMap::new();
    for e in events {
        let days = e.days();
        let (start, end) = (days.start.max(shown.start), days.end.min(shown.end));
        for day in (DateRange { start, end }).days() {
            let m = marks.entry(day).or_default();
            m.count += 1;
            if let Err(at) = m.calendars.binary_search(&e.calendar) {
                m.calendars.insert(at, e.calendar);
            }
        }
    }
    marks
}

/// Accessible name of a day: the full date and the event count.
pub fn day_name(date: NaiveDate, marks: Option<&DayMarks>) -> String {
    let count = match marks.map_or(0, |m| m.count) {
        0 => "no events".to_string(),
        1 => "1 event".to_string(),
        n => format!("{n} events"),
    };
    format!("{}, {count}", date.format("%A, %B %-d, %Y"))
}

/// Gap between month columns.
pub const MONTH_GAP: f32 = 24.;
/// Gap between month rows (mock v8 `.yg` row gap).
pub const ROW_GAP: f32 = 16.;

/// Month columns for a grid `width` px wide: four while each month keeps 220px (gaps included),
/// then three, two, one.
pub fn columns_for(width: f32) -> usize {
    const MONTH_MIN: f32 = 220.;
    (((width + MONTH_GAP) / (MONTH_MIN + MONTH_GAP)) as usize).clamp(1, MAX_COLS)
}

/// `(row, column)` of `month` (1..=12) in a grid of `cols` columns, filled row by row.
pub fn month_cell(month: u32, cols: usize) -> (usize, usize) {
    let ix = month as usize - 1;
    (ix / cols, ix % cols)
}

pub fn month_at(row: usize, col: usize, cols: usize) -> Option<u32> {
    let ix = row * cols + col;
    (col < cols && ix < 12).then_some(ix as u32 + 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
    /// First and last day of the week row (on a title: January and December).
    Home,
    End,
    /// The same day a month back or forward (on a title: the neighbouring month).
    PageUp,
    PageDown,
}

/// The one roving stop of the grid: a month title or a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    /// A month title: the year and the month, so a stale one is told from the shown year's.
    Title(i32, u32),
    Day(NaiveDate),
}

impl Cursor {
    /// The month the stop sits in.
    pub fn month(self) -> u32 {
        match self {
            Cursor::Title(_, m) => m,
            Cursor::Day(d) => d.month(),
        }
    }
}

fn day_in_month(year: i32, month: u32, day: u32) -> NaiveDate {
    (1..=day).rev().find_map(|d| NaiveDate::from_ymd_opt(year, month, d)).expect("day 1 exists")
}

/// One key step, clamped to `year`. Sideways a day (a month on a title), a week up or down; up from
/// the first week row reaches the month title and down from it the month's first day.
pub fn step_cursor(cur: Cursor, dir: Dir, year: i32, ws: Weekday) -> Cursor {
    let r = year_range(year);
    let last = r.end - Duration::days(1);
    match cur {
        Cursor::Title(_, m) => Cursor::Title(year, match dir {
            Dir::Left | Dir::PageUp => m.saturating_sub(1).max(1),
            Dir::Right | Dir::PageDown => (m + 1).min(12),
            Dir::Home => 1,
            Dir::End => 12,
            Dir::Up => m,
            Dir::Down => return Cursor::Day(day_in_month(year, m, 1)),
        }),
        Cursor::Day(d) => {
            let first_row = d < month_grid_range(year, d.month(), ws).start + Duration::days(7);
            let by = |n: i64| Cursor::Day((d + Duration::days(n)).clamp(r.start, last));
            // Home and End stay in d's month: its other-month days are only fillers in this grid.
            let (first, last_of_month) = (day_in_month(year, d.month(), 1), day_in_month(year, d.month(), 31));
            let in_month = |n: i64| Cursor::Day((d + Duration::days(n)).clamp(first, last_of_month));
            match dir {
                Dir::Left => by(-1),
                Dir::Right => by(1),
                Dir::Up if first_row => Cursor::Title(year, d.month()),
                Dir::Up => by(-7),
                Dir::Down => by(7),
                Dir::Home => in_month(-(d.weekday().days_since(ws) as i64)),
                Dir::End => in_month(6 - d.weekday().days_since(ws) as i64),
                Dir::PageUp => Cursor::Day(day_in_month(year, d.month().saturating_sub(1).max(1), d.day())),
                Dir::PageDown => Cursor::Day(day_in_month(year, (d.month() + 1).min(12), d.day())),
            }
        }
    }
}

/// The roving stop shown in `year`: the remembered one, else the anchor, else January 1.
pub fn cursor_in(roving: Option<Cursor>, anchor: NaiveDate, year: i32) -> Cursor {
    match roving {
        Some(Cursor::Day(d)) if d.year() == year => Cursor::Day(d),
        Some(t @ Cursor::Title(y, _)) if y == year => t,
        _ => Cursor::Day(if anchor.year() == year { anchor } else { year_range(year).start }),
    }
}

/// Height of one day: the date box and the dot row at the least, so a crowded window
/// scrolls instead of squashing the date; up to the mock's 30 when there is room.
pub fn day_height(avail: f32, rows: usize) -> f32 {
    (avail / rows.max(1) as f32).clamp(DATE_BOX + DOT_PX, DAY_MAX_H)
}
