//! Month view math: which weeks a scroll offset shows, which month names the header, how a
//! week's events pack into aligned rows, and reading-order movement. No GPUI import.

use std::collections::HashMap;
use std::ops::Range;

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::dates::{DateRange, format_time, week_start_of};
use super::event::Event;

pub type YearMonth = (i32, u32);

/// A finite run of consecutive weeks. The month list is "infinite" only within it.
// ponytail: 10 years each side of the start date; re-center the run when the viewer nears an end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weeks {
    origin: NaiveDate,
    count: usize,
}

impl Weeks {
    pub fn around(center: NaiveDate, ws: Weekday, each_side: usize) -> Self {
        let origin = week_start_of(center, ws) - Duration::weeks(each_side as i64);
        Self { origin, count: 2 * each_side + 1 }
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn week_start(&self) -> Weekday {
        self.origin.weekday()
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn start_of(&self, ix: usize) -> NaiveDate {
        self.origin + Duration::weeks(ix as i64)
    }
    /// Index of the week holding `d`, which may lie outside the run.
    pub fn raw_index(&self, d: NaiveDate) -> i64 {
        (d - self.origin).num_days().div_euclid(7)
    }
    /// Index of the week holding `d`, clamped to the run.
    pub fn index_of(&self, d: NaiveDate) -> usize {
        self.raw_index(d).clamp(0, self.count as i64 - 1) as usize
    }
    /// The weeks named by `(y, m)` (see `month_of_week`), unclamped: the week of the 1st up to the one before the next month's.
    fn named_by_raw(&self, (y, m): YearMonth) -> (i64, i64) {
        let first = |y, m| self.raw_index(NaiveDate::from_ymd_opt(y, m, 1).expect("valid year/month"));
        let next = if m == 12 { first(y + 1, 1) } else { first(y, m + 1) };
        let from = first(y, m);
        (from, (next - 1).max(from))
    }
    /// The week holding the 1st of the month: where navigating to that month scrolls to.
    pub fn first_week_of(&self, (y, m): YearMonth) -> usize {
        self.index_of(NaiveDate::from_ymd_opt(y, m, 1).expect("valid year/month"))
    }
    /// Weeks wholly inside `[offset, offset + viewport)`.
    pub fn fully_visible(&self, offset: f32, viewport: f32, row_h: f32) -> Range<usize> {
        let first = ((offset.max(0.) / row_h).ceil() as usize).min(self.count);
        let last = (((offset.max(0.) + viewport) / row_h).floor() as usize).min(self.count);
        first..last.max(first)
    }
    /// Weeks touching `[offset, offset + viewport)` (pixels scrolled down from the top).
    pub fn visible(&self, offset: f32, viewport: f32, row_h: f32) -> Range<usize> {
        let at = |px: f32| (px.max(0.) / row_h) as usize;
        let first = at(offset).min(self.count);
        let last = ((offset.max(0.) + viewport) / row_h).ceil() as usize;
        first..last.min(self.count).max(first)
    }
    /// The topmost week that is at least half visible.
    pub fn top_week(&self, offset: f32, row_h: f32) -> usize {
        let ix = ((offset - row_h / 2.) / row_h).ceil().max(0.) as usize;
        ix.min(self.count.saturating_sub(1))
    }
    /// The last week that can sit at the top of a viewport: the list cannot scroll past its end.
    /// A viewport of 0 (not laid out yet) knows no limit but the last week.
    pub fn last_top(&self, viewport: f32, row_h: f32) -> usize {
        self.top_week((self.count as f32 * row_h - viewport).max(0.), row_h)
    }
    /// The weeks `days` touches, clipped to the run (empty if it lies wholly outside).
    pub fn span(&self, days: &DateRange) -> Range<usize> {
        let week = |d: NaiveDate| (d - self.origin).num_days().div_euclid(7);
        let first = week(days.start).clamp(0, self.count as i64);
        let end = (week(days.end - Duration::days(1)) + 1).clamp(first, self.count as i64);
        first as usize..end as usize
    }
    /// The month a week is named by: the month of its last day.
    pub fn month_of_week(&self, ix: usize) -> YearMonth {
        let last = self.start_of(ix) + Duration::days(6);
        (last.year(), last.month())
    }
    pub fn title_month(&self, offset: f32, row_h: f32) -> YearMonth {
        self.month_of_week(self.top_week(offset, row_h))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    Idle,
    /// Navigation changed the month: scroll the list to this week.
    Scroll(usize),
    /// The viewer scrolled into another month: the header and mini month follow.
    Follow(YearMonth),
}

/// Keeps the list position and the model's date from fighting: one side leads at a time.
#[derive(Debug, Default)]
pub struct SyncState {
    seen: Option<NaiveDate>,
    /// The requested top week, before clamping to the run or to what can scroll.
    want: Option<i64>,
    /// The list was laid out when the scroll was last requested, so a miss now is the viewer's doing.
    asked: bool,
    /// The week the list rests on after a target past either end was clamped: the anchor stays the user's.
    pinned: Option<usize>,
}

impl SyncState {
    /// `last_top` is `Weeks::last_top` once the list has a viewport, `None` before. A requested
    /// scroll is repeated until the list lands on it (clamped to the run and to `last_top`); it is
    /// dropped only when a laid-out list was asked and the viewer is elsewhere. A clamped landing
    /// leaves the anchor alone; the viewer scrolling away from it gives the lead back.
    pub fn step(&mut self, anchor: NaiveDate, top_week: usize, last_top: Option<usize>, weeks: &Weeks) -> Plan {
        let clamp = |raw: i64| (raw.clamp(0, weeks.len() as i64 - 1) as usize).min(last_top.unwrap_or(usize::MAX));
        let month = |d: NaiveDate| (d.year(), d.month());
        if self.seen != Some(anchor) {
            let (first, last) = weeks.named_by_raw(month(anchor));
            // Another month: its first week. Another day of the same month: its week, kept low enough
            // that the title still names this month.
            let raw = match self.seen {
                Some(was) if month(was) == month(anchor) => weeks.raw_index(anchor).clamp(first, last),
                _ => first,
            };
            (self.seen, self.want, self.asked, self.pinned) = (Some(anchor), Some(raw), false, None);
            return Plan::Scroll(clamp(raw));
        }
        if let Some(raw) = self.want {
            let ix = clamp(raw);
            if top_week == ix {
                self.want = None;
                self.pinned = (raw != ix as i64).then_some(ix);
                return Plan::Idle;
            }
            if last_top.is_none() || !self.asked {
                self.asked = last_top.is_some();
                return Plan::Scroll(ix);
            }
            self.want = None;
        }
        if self.pinned == Some(top_week) {
            return Plan::Idle;
        }
        self.pinned = None;
        let shown = weeks.month_of_week(top_week);
        if shown == month(anchor) {
            return Plan::Idle;
        }
        Plan::Follow(shown)
    }

    /// The model took a `Plan::Follow`: its anchor is now `anchor`, which is not a navigation.
    pub fn followed(&mut self, anchor: NaiveDate) {
        self.seen = Some(anchor);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub id: u64,
    /// Position of the event in the slice given to `layout_week`.
    pub ix: usize,
    pub lane: usize,
    pub col: usize,
    pub span: usize,
    pub from_prev: bool,
    pub to_next: bool,
    /// All-day or multi-day: drawn as a tinted bar; otherwise a single-day row.
    pub pill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeekLayout {
    pub items: Vec<Placed>,
    /// Items past the cap: they show only as a count in "+n more".
    pub hidden: Vec<Placed>,
    /// Hidden items per day column ("+n more").
    pub more: [usize; 7],
}

pub fn layout_week(start: NaiveDate, events: &[&Event], cap: usize) -> WeekLayout {
    let end = start + Duration::days(7);
    let mut found: Vec<(Placed, NaiveDate, i64, chrono::NaiveDateTime)> = events
        .iter()
        .enumerate()
        .filter_map(|(ix, e)| {
            let days = e.days();
            if days.end <= start || days.start >= end {
                return None;
            }
            let (from, to) = (days.start.max(start), days.end.min(end));
            let placed = Placed {
                id: e.id,
                ix,
                lane: 0,
                col: (from - start).num_days() as usize,
                span: (to - from).num_days() as usize,
                from_prev: days.start < start,
                to_next: days.end > end,
                pill: e.all_day || days.len() > 1,
            };
            Some((placed, days.start, days.len() as i64, e.start))
        })
        .collect();
    // Bars first so they hold the top rows; then by the day the event really starts (not where this
    // week cuts it), longer first: every week orders the pieces of one bar the same way.
    found.sort_by_key(|(p, from, len, at)| (!p.pill, *from, std::cmp::Reverse(*len), *at, p.id));

    let mut busy: Vec<u8> = Vec::new(); // per lane, a bit per day column
    let mut out = WeekLayout::default();
    for (mut p, ..) in found {
        let mask = (((1u16 << p.span) - 1) << p.col) as u8;
        let lane = busy.iter().position(|b| b & mask == 0).unwrap_or(busy.len());
        if lane == busy.len() {
            busy.push(0);
        }
        busy[lane] |= mask;
        p.lane = lane;
        if lane < cap {
            out.items.push(p);
        } else {
            (p.col..p.col + p.span).for_each(|c| out.more[c] += 1);
            out.hidden.push(p);
        }
    }
    out
}

/// Page padding on both sides plus the gap between the sidebar and the calendar (`ui::shell`).
const PAGE_CHROME: f32 = 48.;
pub const SIDEBAR_W: f32 = 236.;
pub const RAIL_W: f32 = 64.;

/// The month grid's width in a window: what is left beside the sidebar (or the rail).
pub fn grid_width(window: f32, rail: bool) -> f32 {
    window - PAGE_CHROME - if rail { RAIL_W } else { SIDEBAR_W }
}

/// Grid width from which cells show three rows: that of the 1120 window, the widest with a rail.
pub const WIDE_GRID: f32 = 1120. - PAGE_CHROME - SIDEBAR_W;

/// Rows a cell shows: two on a narrow grid, three on a wide one.
pub fn cap_for(grid_width: f32) -> usize {
    if grid_width < WIDE_GRID { 2 } else { 3 }
}

/// The events of each week they touch (as indices into `events`), found in one pass.
pub fn bucket_by_week(weeks: &Weeks, events: &[Event]) -> HashMap<usize, Vec<usize>> {
    let mut buckets: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, e) in events.iter().enumerate() {
        for ix in weeks.span(&e.days()) {
            buckets.entry(ix).or_default().push(i);
        }
    }
    buckets
}

/// What a screen reader says for an item: title, time, calendar, the dates of a multi-day one and
/// which piece of it this week shows.
pub fn item_name(e: &Event, calendar: &str, hour24: bool, p: &Placed) -> String {
    let (days, title) = (e.days(), &e.title);
    let day = |d: NaiveDate| d.format("%b %-d");
    let time = |t| format_time(t, hour24);
    let mut name = match (e.all_day, days.len() > 1) {
        (true, false) => format!("{title}, {calendar}, all day"),
        (true, true) => format!("{title}, {calendar}, all day, {} to {}", day(days.start), day(days.end - Duration::days(1))),
        (false, false) => format!("{title}, {}, {calendar}", time(e.start.time())),
        (false, true) => format!("{title}, {} {} to {} {}, {calendar}", time(e.start.time()), day(days.start), time(e.end.time()), day(e.end.date())),
    };
    if p.from_prev {
        name.push_str(", continues from the previous week");
    }
    if p.to_next {
        name.push_str(", continues into the next week");
    }
    name
}

pub fn reading_order(events: &[&Event]) -> Vec<u64> {
    let mut v: Vec<_> = events.iter().map(|e| (e.start, e.id)).collect();
    v.sort();
    v.into_iter().map(|(_, id)| id).collect()
}

/// The next (or previous) id in reading order; stays put at either end. `None` if `current` is not in it.
pub fn neighbor(order: &[u64], current: u64, forward: bool) -> Option<u64> {
    let at = order.iter().position(|&id| id == current)?;
    let ix = if forward { (at + 1).min(order.len() - 1) } else { at.saturating_sub(1) };
    Some(order[ix])
}

/// Where an arrow lands from nowhere: forward, the first event of the month on screen (or the next
/// one with any); back, the last event of it (or the nearest before).
pub fn enter(events: &[&Event], month: YearMonth, forward: bool) -> Option<u64> {
    let ym = |e: &Event| (e.start.year(), e.start.month());
    let keyed = |e: &&Event| (e.start, e.id);
    let found = if forward { events.iter().filter(|e| ym(e) >= month).map(keyed).min() } else { events.iter().filter(|e| ym(e) <= month).map(keyed).max() };
    found.map(|(_, id)| id)
}
