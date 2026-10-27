use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use super::event::{Event, MINUTES_PER_DAY};
use super::layout::{SNAP_MINUTES, snap, snap_ceil, snap_floor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pos {
    /// Time grid: day column and minute-of-day (unsnapped, clamped to 0..=1440).
    Grid(NaiveDate, i32),
    AllDay(NaiveDate),
    Month(NaiveDate),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub all_day: bool,
}

#[derive(Debug, Clone)]
enum Mode {
    Create,
    Move(Event),
    Resize(Event),
}

/// One in-progress pointer drag. Hold an `Option<Drag>`: `None` is idle, drop it to cancel.
/// The preview is recomputed from the press and current positions every time, so a drag can
/// pass through the all-day row and back without losing the original event.
#[derive(Debug, Clone)]
pub struct Drag {
    mode: Mode,
    press: Pos,
    cur: Pos,
}

const DEFAULT_START: NaiveTime = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
const DEFAULT_LEN: i64 = 60;

fn day_of(p: Pos) -> NaiveDate {
    match p {
        Pos::Grid(d, _) | Pos::AllDay(d) | Pos::Month(d) => d,
    }
}

fn at_minute(day: NaiveDate, minute: i32) -> NaiveDateTime {
    day.and_time(NaiveTime::MIN) + Duration::minutes(minute.clamp(0, MINUTES_PER_DAY as i32) as i64)
}

fn midnight(d: NaiveDate) -> NaiveDateTime {
    d.and_time(NaiveTime::MIN)
}

/// Snap to the 15-minute grid, measured from the datetime's own midnight.
fn snap_dt(dt: NaiveDateTime) -> NaiveDateTime {
    let m = (dt - midnight(dt.date())).num_minutes();
    midnight(dt.date()) + Duration::minutes(snap(m, SNAP_MINUTES))
}

fn all_day_preview(first: NaiveDate, days: i64) -> Preview {
    Preview { start: midnight(first), end: midnight(first + Duration::days(days)), all_day: true }
}

impl Drag {
    pub fn create(at: Pos) -> Self {
        Self { mode: Mode::Create, press: at, cur: at }
    }

    pub fn move_event(e: &Event, at: Pos) -> Self {
        Self { mode: Mode::Move(e.clone()), press: at, cur: at }
    }

    /// Bottom-edge resize on the time grid; only timed events can resize.
    pub fn resize_bottom(e: &Event, at: Pos) -> Option<Self> {
        (!e.all_day).then(|| Self { mode: Mode::Resize(e.clone()), press: at, cur: at })
    }

    pub fn update(&mut self, at: Pos) {
        self.cur = at;
    }

    /// True once the pointer left the press position's cell (click vs drag). A grid cell is a
    /// 15-minute slot; the all-day row and month cells are the same per-date cell.
    /// Move and resize previews are a step function of this: identity inside the press cell,
    /// then they follow the pointer.
    pub fn moved(&self) -> bool {
        let cell = |p: Pos| match p {
            Pos::Grid(d, m) => Pos::Grid(d, snap_floor(m as i64, SNAP_MINUTES) as i32),
            Pos::AllDay(d) | Pos::Month(d) => Pos::Month(d),
        };
        cell(self.press) != cell(self.cur)
    }

    /// Pointer release. `Some(preview)` is the commit; `None` means it was a click (the pointer
    /// never left the press cell), so the caller decides what a click does. Cancel (Esc) is
    /// dropping the `Drag` without calling this.
    pub fn end(self) -> Option<Preview> {
        // Released on a surface the mode can't commit to is a cancel, like its preview.
        // Move keeps grid -> all-day row (timed -> all-day) and all-day <-> grid conversions.
        let on_grid = |p: Pos| matches!(p, Pos::Grid(..));
        let cancel = match self.mode {
            Mode::Create => on_grid(self.press) != on_grid(self.cur),
            Mode::Resize(_) => !on_grid(self.cur),
            Mode::Move(_) => on_grid(self.press) && matches!(self.cur, Pos::Month(_)),
        };
        if cancel {
            return None;
        }
        self.moved().then(|| self.preview())
    }

    pub fn preview(&self) -> Preview {
        match &self.mode {
            Mode::Create => self.create_preview(),
            Mode::Move(e) if self.moved() => self.move_preview(e),
            Mode::Resize(e) if self.moved() => self.resize_preview(e),
            Mode::Move(e) | Mode::Resize(e) => Preview { start: e.start, end: e.end, all_day: e.all_day },
        }
    }

    /// The dragged event with the preview applied (identity fields untouched).
    pub fn apply(&self, e: &Event) -> Event {
        let p = self.preview();
        Event { start: p.start, end: p.end, all_day: p.all_day, ..e.clone() }
    }

    fn create_preview(&self) -> Preview {
        match self.press {
            Pos::Grid(day, a) => {
                let c = match self.cur {
                    Pos::Grid(d, _) if d > day => MINUTES_PER_DAY as i32,
                    Pos::Grid(d, _) if d < day => 0,
                    Pos::Grid(_, c) => c,
                    _ => a, // other surfaces are ignored; other columns clamp to this day's edges
                };
                let max = MINUTES_PER_DAY as i64;
                let (lo, hi) = (a.min(c).clamp(0, max as i32) as i64, a.max(c).clamp(0, max as i32) as i64);
                let lo = snap_floor(lo, SNAP_MINUTES).min(max - SNAP_MINUTES as i64);
                let hi = snap_ceil(hi, SNAP_MINUTES).max(lo + SNAP_MINUTES as i64).min(max);
                Preview { start: at_minute(day, lo as i32), end: at_minute(day, hi as i32), all_day: false }
            }
            press => {
                let cur = match self.cur {
                    Pos::Grid(..) => press,
                    p => p,
                };
                let (a, b) = (day_of(press), day_of(cur));
                all_day_preview(a.min(b), (a.max(b) - a.min(b)).num_days() + 1)
            }
        }
    }

    fn move_preview(&self, e: &Event) -> Preview {
        let shift = Duration::days((day_of(self.cur) - day_of(self.press)).num_days());
        let dur = e.end - e.start;
        match self.cur {
            Pos::Grid(day, _) if e.all_day => {
                let start = day.and_time(DEFAULT_START);
                Preview { start, end: start + Duration::minutes(DEFAULT_LEN), all_day: false }
            }
            Pos::Grid(day, m) => {
                // keep the grab offset inside the event, then snap the new start
                let grab = match self.press {
                    Pos::Grid(d, pm) => at_minute(d, pm),
                    p => day_of(p).and_time(e.start.time()),
                };
                let start = snap_dt(at_minute(day, m) - (grab - e.start));
                Preview { start, end: start + dur, all_day: false }
            }
            // Only a press on the time grid converts; a timed event already shown in the row keeps its times.
            Pos::AllDay(_) if !e.all_day && matches!(self.press, Pos::Grid(..)) => all_day_preview(e.start.date() + shift, e.days().len() as i64),
            Pos::AllDay(_) | Pos::Month(_) => Preview { start: e.start + shift, end: e.end + shift, all_day: e.all_day },
        }
    }

    fn resize_preview(&self, e: &Event) -> Preview {
        let end = match self.cur {
            Pos::Grid(day, m) => snap_dt(at_minute(day, m)).max(e.start + Duration::minutes(SNAP_MINUTES as i64)),
            _ => e.end,
        };
        Preview { start: e.start, end, all_day: false }
    }
}
