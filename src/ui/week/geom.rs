//! Week and day geometry: pixels to minutes and days, block and all-day layout, hit testing, scroll
//! targets. Pure: no GPUI import, no clock. The views draw what this lays out and hit-test against it.

use std::collections::HashMap;

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};

use crate::domain::dates::format_time;
use crate::domain::drag::Pos;
use crate::domain::event::{Event, MINUTES_PER_DAY};
use crate::domain::layout::pack_columns;
use crate::domain::month::layout_week;

/// Guide 5: the hour row is 44, the all-day row 30.
pub const HOUR_H: f32 = 44.;
pub const GRID_H: f32 = 24. * HOUR_H;
pub const GUTTER_W: f32 = 52.;
pub const ALL_DAY_ROW_H: f32 = 30.;
pub const LANE_H: f32 = 22.;
const LANES_PAD: f32 = 4.;
/// The grab zone for a bottom-edge resize.
pub const EDGE_PX: f32 = 6.;
/// A 15-minute block is 11px tall; it is drawn at least this tall so its title can be read.
pub const MIN_BLOCK_H: f32 = 18.;
/// The minutes that height covers; a shorter event is packed and hit-tested as if it lasted this long.
const MIN_BLOCK_MIN: u32 = (MIN_BLOCK_H * 60. / HOUR_H).ceil() as u32;
pub const MORE_H: f32 = 20.;
/// The strip at the right of a crowded cluster where its "+n more" chip sits, as a fraction of the column.
pub const MORE_X: (f32, f32) = (0.75, 1.);
/// Guide 6: at most two events across; further overlaps collapse into a "+n more" block.
pub const MAX_ACROSS: usize = 2;
/// The grid opens here when today is not shown.
const OPEN_AT_MIN: i32 = 8 * 60;
const EDGE_ZONE: f32 = 32.;
const EDGE_MAX_STEP: f32 = 20.;

pub fn minute_to_y(m: i32) -> f32 {
    m as f32 * HOUR_H / 60.
}

/// Unsnapped minute-of-day under a grid y. Midnight (1440) is only an end position: a start stops at 1439.
pub fn y_to_minute(y: f32, end: bool) -> i32 {
    ((y * 60. / HOUR_H).floor() as i32).clamp(0, MINUTES_PER_DAY as i32 - i32::from(!end))
}

/// The pointer's frame of reference, relative to the top-left of the all-day row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    /// Gutter plus day columns.
    pub width: f32,
    /// Height of the scrolling time grid below the all-day row.
    pub view_h: f32,
    pub all_day_h: f32,
    pub scroll: f32,
}

/// Where a pointer at `(x, y)` is. `clamp` (a drag in progress) pulls a pointer that left the columns back to
/// the nearest column and may end at midnight; a press outside the columns or below the day hits nothing.
pub fn hit(a: &Area, days: &[NaiveDate], x: f32, y: f32, clamp: bool) -> Option<Pos> {
    let cols = days.len();
    let cx = x - GUTTER_W;
    if !clamp && (cx < 0. || cx >= a.width - GUTTER_W || y < 0. || y >= a.all_day_h + a.view_h) {
        return None;
    }
    let day = days[column_of(a, cols, x).0];
    if y < a.all_day_h {
        return Some(Pos::AllDay(day));
    }
    let content = y - a.all_day_h + a.scroll;
    (clamp || content < GRID_H).then(|| Pos::Grid(day, y_to_minute(content, clamp)))
}

/// How far a drag may reach into the all-day row, by where it began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Resize, or a create that began on the grid: the grid only.
    Grid,
    /// Moving a timed event: the row, but not the header above it.
    Row,
    /// Began in the row: it stays there.
    Any,
}

/// `hit` for a drag in progress: a pointer over the all-day row stays on the grid (at the scrolled-to time,
/// minute 0 at the top of the day) unless `reach` lets it convert.
pub fn drag_hit(a: &Area, days: &[NaiveDate], x: f32, y: f32, reach: Reach) -> Option<Pos> {
    match hit(a, days, x, y, true)? {
        Pos::AllDay(day) if reach == Reach::Grid || (reach == Reach::Row && y < 0.) => Some(Pos::Grid(day, y_to_minute(y - a.all_day_h + a.scroll, true))),
        pos => Some(pos),
    }
}

/// The day column under `x` and how far across it (0..1); a pointer outside the columns gets the nearest one.
pub fn column_of(a: &Area, cols: usize, x: f32) -> (usize, f32) {
    let col_w = (a.width - GUTTER_W) / cols as f32;
    let at = ((x - GUTTER_W) / col_w).clamp(0., cols as f32 - 1e-4);
    (at.floor() as usize, at.fract())
}

/// The scroll that brings `top..top + h` into a viewport of `view_h` showing from `scroll`, moving as little as possible.
pub fn scroll_into_view(top: f32, h: f32, scroll: f32, view_h: f32) -> f32 {
    if top < scroll {
        top
    } else if top + h > scroll + view_h {
        (top + h - view_h).min(top)
    } else {
        scroll
    }
    .clamp(0., (GRID_H - view_h).max(0.))
}

/// A timed event's piece on one day. `col` of `across` side-by-side columns, left of the `right` fraction of
/// the column kept free for the cluster's "+n more" chip. Only the piece that `ends` the event can be resized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub id: u64,
    pub top: f32,
    pub h: f32,
    pub col: usize,
    pub across: usize,
    pub right: f32,
    pub ends: bool,
}

impl Block {
    /// Left and right edge as fractions of the day column.
    pub fn x(&self) -> (f32, f32) {
        let w = (1. - self.right) / self.across as f32;
        (self.col as f32 * w, (self.col + 1) as f32 * w)
    }
}

/// The "+n more" chip: events past the second column of a cluster. Drawn in the strip `MORE_X` that the
/// cluster's blocks leave free.
#[derive(Debug, Clone, PartialEq)]
pub struct More {
    pub top: f32,
    /// At most `MORE_H`, and within the first hidden event's own rows, so inside its cluster.
    pub h: f32,
    pub n: usize,
    pub ids: Vec<u64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DayLayout {
    pub blocks: Vec<Block>,
    pub more: Vec<More>,
}

pub fn layout_day(day: NaiveDate, events: &[&Event]) -> DayLayout {
    let day_end = day.and_time(NaiveTime::MIN) + Duration::days(1);
    // Packed by the rows each event is drawn over: a short one is `MIN_BLOCK_H` tall, so it takes its neighbours' room.
    let segs: Vec<(u64, u32, u32)> =
        events.iter().filter_map(|e| e.segment_on(day).map(|(s, t)| (e.id, s, t.max(s + MIN_BLOCK_MIN).min(MINUTES_PER_DAY)))).collect();
    let span: HashMap<u64, (u32, u32)> = segs.iter().map(|&(id, s, t)| (id, (s, t))).collect();
    // The drawn height: the true one, at least `MIN_BLOCK_H`, within the rows it was packed for.
    let true_end: HashMap<u64, u32> = events.iter().filter_map(|e| e.segment_on(day).map(|(_, t)| (e.id, t))).collect();
    let ends: HashMap<u64, bool> = events.iter().map(|e| (e.id, e.end <= day_end)).collect();
    let placed = pack_columns(&segs);
    // `pack_columns` lists a cluster of transitively overlapping events together; find where each ends.
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    let mut end = 0;
    for (i, p) in placed.iter().enumerate() {
        let (s, t) = span[&p.id];
        if clusters.is_empty() || s >= end {
            clusters.push(Vec::new());
            end = 0;
        }
        end = end.max(t);
        clusters.last_mut().expect("just pushed").push(i);
    }
    let mut out = DayLayout::default();
    for cluster in clusters {
        let mut hidden: Vec<(u32, u32, u64)> = Vec::new();
        let from = out.blocks.len();
        for &i in &cluster {
            let p = placed[i];
            let (s, t) = span[&p.id];
            if p.col >= MAX_ACROSS {
                hidden.push((s, t, p.id));
                continue;
            }
            let top = minute_to_y(s as i32);
            out.blocks.push(Block { id: p.id, top, h: (minute_to_y(true_end[&p.id] as i32) - top).max(MIN_BLOCK_H).min(minute_to_y(t as i32) - top), col: p.col, across: p.cols.min(MAX_ACROSS), right: 0., ends: ends[&p.id] });
        }
        if let Some(&(s, t, _)) = hidden.iter().min_by_key(|h| h.0) {
            let top = minute_to_y(s as i32);
            out.more.push(More { top, h: MORE_H.min(minute_to_y(t as i32) - top), n: hidden.len(), ids: hidden.iter().map(|h| h.2).collect() });
            // Only blocks beside a hidden event give up the strip; the rest of the cluster keeps its width.
            for b in &mut out.blocks[from..] {
                if hidden.iter().any(|&(s, t, _)| b.top < minute_to_y(t as i32) && minute_to_y(s as i32) < b.top + b.h) {
                    b.right = 1. - MORE_X.0;
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Body,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Block(u64, Zone),
    More,
}

/// What is under `(frac, y)`: `frac` is 0..1 across the day column, `y` is grid content pixels.
/// Later blocks are over earlier ones, and all are over the "+n more" chips, as drawn.
pub fn target_at(l: &DayLayout, frac: f32, y: f32) -> Option<Target> {
    let block = l.blocks.iter().rev().find(|b| {
        let (lo, hi) = b.x();
        (lo..hi).contains(&frac) && (b.top..b.top + b.h).contains(&y)
    });
    if let Some(b) = block {
        return Some(Target::Block(b.id, if b.ends && y >= b.top + b.h - EDGE_PX.min(b.h / 2.) { Zone::Bottom } else { Zone::Body }));
    }
    l.more.iter().any(|m| (MORE_X.0..MORE_X.1).contains(&frac) && (m.top..m.top + m.h).contains(&y)).then_some(Target::More)
}

/// An all-day event as a bar over `col..col + span` of the shown days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub id: u64,
    pub col: usize,
    pub span: usize,
    pub lane: usize,
    pub from_prev: bool,
    pub to_next: bool,
}

/// Lanes for the all-day events touching `first..first + cols` (7 for a week, 1 for a day), and the lane count.
pub fn all_day_bars(first: NaiveDate, cols: usize, events: &[&Event]) -> (Vec<Bar>, usize) {
    let end = first + Duration::days(cols as i64);
    let here: Vec<&Event> = events.iter().copied().filter(|e| e.in_all_day_row() && e.days().start < end && e.days().end > first).collect();
    let lay = layout_week(first, &here, usize::MAX);
    let bars: Vec<Bar> = lay
        .items
        .iter()
        .filter(|p| p.col < cols)
        .map(|p| Bar { id: p.id, col: p.col, span: p.span.min(cols - p.col), lane: p.lane, from_prev: p.from_prev, to_next: here[p.ix].days().end > end })
        .collect();
    let lanes = bars.iter().map(|b| b.lane + 1).max().unwrap_or(0);
    (bars, lanes)
}

/// Guide 5 fixes the row at 30; more lanes make it taller rather than clipping a bar.
pub fn all_day_height(lanes: usize) -> f32 {
    (lanes as f32 * LANE_H + 2. * LANES_PAD).max(ALL_DAY_ROW_H)
}

pub fn lane_top(lane: usize) -> f32 {
    LANES_PAD + lane as f32 * LANE_H
}

/// The bar covering column `col` at `y` pixels into the all-day row.
pub fn bar_at(bars: &[Bar], col: usize, y: f32) -> Option<&Bar> {
    bars.iter().find(|b| (b.col..b.col + b.span).contains(&col) && (lane_top(b.lane)..lane_top(b.lane) + LANE_H).contains(&y))
}

/// Grid scroll on open: a third of the way down the window at the current time when today is shown,
/// otherwise 08:00 at the top. Never past either end of the day.
pub fn scroll_target(now_min: Option<i32>, view_h: f32) -> f32 {
    let want = now_min.map_or(minute_to_y(OPEN_AT_MIN), |m| minute_to_y(m) - view_h / 3.);
    want.clamp(0., (GRID_H - view_h).max(0.))
}

/// Pixels to scroll for a pointer `y` inside the grid viewport during a drag: negative near the top, positive
/// near the bottom, faster the closer to the edge, zero away from it.
pub fn edge_scroll(y: f32, view_h: f32) -> f32 {
    let speed = |dist: f32| (EDGE_ZONE - dist.max(0.)) / EDGE_ZONE * EDGE_MAX_STEP;
    if y < EDGE_ZONE {
        -speed(y)
    } else if y > view_h - EDGE_ZONE {
        speed(view_h - y)
    } else {
        0.
    }
}

/// The time range of the piece of `e` drawn on `day`: a multi-day event shows each day's own start and end.
pub fn block_range(e: &Event, day: NaiveDate, hour24: bool) -> String {
    let (s, t) = e.segment_on(day).unwrap_or((0, 0));
    let time = |m: u32| format_time(NaiveTime::MIN + Duration::minutes(i64::from(m % MINUTES_PER_DAY)), hour24);
    // A piece that runs to midnight ends after it starts: "24:00", or "12:00 AM next day" where 24:00 is not written.
    let end = match (t >= MINUTES_PER_DAY, hour24) {
        (false, _) => time(t),
        (true, true) => "24:00".to_string(),
        (true, false) => format!("{} next day", time(t)),
    };
    format!("{} – {end}", time(s))
}

/// Accessible name of a timed block: title, the piece shown on `day`, calendar.
pub fn block_name(e: &Event, day: NaiveDate, calendar: &str, hour24: bool) -> String {
    format!("{}, {}, {calendar}", e.title, block_range(e, day, hour24).replace(" – ", " to "))
}

/// Accessible name of an all-day bar: title, time range, calendar.
pub fn event_name(e: &Event, calendar: &str, hour24: bool) -> String {
    let (title, day) = (&e.title, |d: NaiveDate| d.format("%b %-d"));
    if e.all_day {
        let days = e.days();
        return match days.len() > 1 {
            false => format!("{title}, {calendar}, all day"),
            true => format!("{title}, {calendar}, all day, {} to {}", day(days.start), day(days.end - Duration::days(1))),
        };
    }
    let time = |dt: NaiveDateTime| format_time(dt.time(), hour24);
    if e.start.date() == e.end.date() || (e.end.time() == NaiveTime::MIN && e.end.date() == e.start.date() + Duration::days(1)) {
        format!("{title}, {} to {}, {calendar}", time(e.start), time(e.end))
    } else {
        format!("{title}, {} {} to {} {}, {calendar}", time(e.start), day(e.start.date()), time(e.end), day(e.end.date()))
    }
}
