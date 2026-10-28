//! Shell state and the actions on it. No GPUI import: the views call these methods and tests
//! drive them directly. Every appearance or visibility change is applied to the store and saved.

mod events;
mod keys;
mod merge;
mod month;
mod week;
mod year;

pub use keys::{Command, command};
pub use month::NEW_EVENT_TITLE;

use std::io;
use std::path::PathBuf;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use crate::domain::dates::{add_months, month_grid_range};
use crate::store::settings::{Accent, CalendarColor, Mode, Neutral};
use crate::store::{CALENDAR_NAMES, LoadNote, PERSONAL, Saved, Store, WORK};
use crate::domain::event::CalendarId;
use crate::domain::year::Cursor;
use crate::theme::resolve::{Theme, resolve};
use crate::theme::rules::{accent_unavailable_reason, calendar_slots, slot_unavailable_reason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Day,
    Week,
    Month,
    Year,
}

impl View {
    pub const ALL: [View; 4] = [View::Day, View::Week, View::Month, View::Year];

    pub fn name(self) -> &'static str {
        match self {
            View::Day => "Day",
            View::Week => "Week",
            View::Month => "Month",
            View::Year => "Year",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Calendar,
    Appearance,
}

/// The "Next up" banner content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextUp {
    pub id: u64,
    pub title: String,
    pub relative: String,
    pub clock: String,
}

impl NextUp {
    /// Visible label and accessible name of the banner button.
    pub fn label(&self) -> &'static str {
        "Open event"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniDay {
    pub date: NaiveDate,
    /// Accessible name: the full date, e.g. "October 28".
    pub name: String,
    pub is_today: bool,
    pub selected: bool,
    pub out_of_month: bool,
}

impl MiniDay {
    /// Today gets the accent circle, whether or not it is also selected.
    pub fn accent_fill(&self) -> bool {
        self.is_today
    }

    /// The selected day gets the ink ring, on top of the accent circle if it is today.
    pub fn ring(&self) -> bool {
        self.selected
    }
}

pub struct Model {
    pub store: Store,
    path: Option<PathBuf>,
    pub view: View,
    /// The date the view shows: `base` moved by `steps` view units (derived, so months never drift).
    pub anchor: NaiveDate,
    base: NaiveDate,
    steps: i32,
    pub today: NaiveDate,
    pub now: NaiveTime,
    pub screen: Screen,
    /// Status line of the Appearance screen (refusal reasons, save errors).
    pub hint: String,
    /// Save problems and a blocked store; shown on every screen until dismissed. A later good
    /// save does not clear it. Empty when all is well.
    pub notice: String,
    /// Why the store was not read, while saving is off.
    block_reason: Option<String>,
    /// The store when saving went off: what "changed since" means for a retry.
    blocked_base: Option<Store>,
    /// The day of month the viewer chose (the 31st stays wanted while February shows the 28th).
    want_day: u32,
    pub system_dark: bool,
    /// First day of the month the mini month shows.
    mini: NaiveDate,
    /// The stop the Year view's arrow keys are on; any change of what is shown clears it.
    pub year_cursor: Option<Cursor>,
    /// Counts the resets of `year_cursor`: the Year view scrolls back to the anchor once per count.
    year_epoch: u32,
}

fn month_start(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("day 1 exists")
}

/// Said at startup and on every save when there is nowhere to keep the settings.
pub const NO_PATH: &str = "Changes are not saved: there is no place to keep settings on this computer. They last until you quit.";

/// The notice for a file that was set aside or repaired on load.
pub fn load_note_text(note: LoadNote) -> String {
    match note {
        LoadNote::Quarantined => "The settings file could not be used (damaged or from a newer version). It was kept as a backup next to it (.bak) and defaults are shown.",
        LoadNote::Repaired => "Some saved settings could not be read and were reset. The original file was kept as a backup next to it (.bak).",
    }
    .into()
}

/// How a save ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Save {
    Durable,
    /// Written, but the disk may not have it yet.
    NotDurable,
    Failed,
}

fn blocked_notice(why: &str) -> String {
    format!("Settings file not read ({why}). Changes are not saved; the file was left as it is.")
}

impl Model {
    pub fn new(store: Store, path: Option<PathBuf>, today: NaiveDate, now: NaiveTime, system_dark: bool) -> Self {
        Self {
            store,
            path,
            view: View::Month,
            anchor: today,
            base: today,
            steps: 0,
            today,
            now,
            screen: Screen::Calendar,
            hint: String::new(),
            notice: String::new(),
            block_reason: None,
            blocked_base: None,
            want_day: today.day(),
            system_dark,
            mini: month_start(today),
            year_cursor: None,
            year_epoch: 0,
        }
    }

    pub fn theme(&self) -> Theme {
        resolve(&self.store.settings.appearance, self.system_dark)
    }

    /// The store could not be read: keep the file as it is and never save over it this session.
    pub fn block_saving(&mut self, why: String) {
        self.notice = blocked_notice(&why);
        self.blocked_base = Some(self.store.clone());
        self.block_reason = Some(why);
    }

    /// Whether "Retry loading settings" makes sense: saving is off and there is a file to read.
    pub fn can_retry(&self) -> bool {
        self.block_reason.is_some() && self.path.is_some()
    }

    /// Read the file again. On success saving is back on; what the viewer changed while it was off
    /// (appearance fields, calendar visibility) is kept on top of the file and saved, everything else comes
    /// from the file. Only the "not read" notice is cleared.
    pub fn retry_load(&mut self) {
        let Some(path) = self.path.clone().filter(|_| self.block_reason.is_some()) else { return };
        let (loaded, note) = match Store::load_noted(&path) {
            Ok(r) => r,
            Err(e) => return self.block_saving(e.to_string()),
        };
        let why = self.block_reason.take().unwrap_or_default();
        if self.notice == blocked_notice(&why) {
            self.notice.clear();
        }
        self.hint.clear();
        let base = self.blocked_base.take();
        let mine = std::mem::replace(&mut self.store, loaded);
        let changed = base.is_some_and(|b| self.merge_changes(&b, &mine));
        if let Some(note) = note {
            self.warn(load_note_text(note));
        }
        if changed {
            self.warn("Settings file read. Changes made while saving was off were kept and saved.".into());
            self.persist();
        }
    }

    pub fn dismiss_notice(&mut self) {
        self.notice.clear();
    }

    /// Show `msg` unless an earlier problem is still waiting to be dismissed.
    pub fn warn(&mut self, msg: String) {
        if self.notice.is_empty() {
            self.notice = msg;
        }
    }

    /// Move the clock. `true` when the date changed. A viewer who was looking at today moves with it
    /// (anchor, step base and mini month together); one who navigated elsewhere stays put.
    pub fn set_now(&mut self, now: NaiveDateTime) -> bool {
        let (old, new) = (self.today, now.date());
        self.now = now.time();
        if old == new {
            return false;
        }
        self.today = new;
        if self.anchor == old {
            let mini = self.mini;
            self.focus_anchor(new);
            if mini != month_start(old) {
                self.mini = mini; // the viewer was browsing another month in the mini calendar
            }
        }
        true
    }

    /// Save, and say so when it did not (fully) happen.
    fn persist(&mut self) -> Save {
        if let Some(why) = &self.block_reason {
            let msg = blocked_notice(why);
            self.warn(msg);
            return Save::Failed;
        }
        let Some(p) = &self.path else {
            self.warn(NO_PATH.into());
            return Save::Failed;
        };
        let r = self.store.save(p);
        self.record(r)
    }

    /// Turn the outcome of a save into a notice. `true` only for `Saved::Durable`.
    pub fn record_save(&mut self, r: io::Result<Saved>) -> bool {
        self.record(r) == Save::Durable
    }

    pub fn record(&mut self, r: io::Result<Saved>) -> Save {
        match r {
            Ok(Saved::Durable) => Save::Durable,
            Ok(Saved::NotDurable(e)) => {
                self.warn(format!("Saved, but it may be lost if the computer loses power ({e})."));
                Save::NotDurable
            }
            Err(e) => {
                self.warn(format!("Settings not saved: {e}"));
                Save::Failed
            }
        }
    }

    // --- calendars ---

    pub fn toggle_calendar(&mut self, id: CalendarId) {
        if let Some(c) = self.store.calendars.iter_mut().find(|c| c.id == id) {
            c.visible = !c.visible;
            self.persist();
        }
    }

    // --- appearance ---

    pub fn toggle_appearance(&mut self) {
        self.screen = if self.screen == Screen::Appearance { Screen::Calendar } else { Screen::Appearance };
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.hint.clear();
        self.store.settings.appearance.mode = mode;
        self.persist();
    }

    pub fn set_neutral(&mut self, neutral: Neutral) {
        self.hint.clear();
        self.store.settings.appearance.neutral = neutral;
        self.persist();
    }

    pub fn accent_reason(&self, a: Accent) -> Option<String> {
        accent_unavailable_reason(&self.store.settings.appearance, a)
    }

    /// Accessible name of an accent swatch: name, plus "selected" or why it is unavailable.
    pub fn accent_name(&self, a: Accent) -> String {
        match self.accent_reason(a) {
            Some(r) => format!("{a:?}, unavailable: {r}"),
            None if self.store.settings.appearance.accent == a => format!("{a:?}, selected"),
            None => format!("{a:?}"),
        }
    }

    /// `Err(reason)` when the swatch is unavailable; the reason is also shown in the status line.
    pub fn set_accent(&mut self, a: Accent) -> Result<(), String> {
        if let Some(r) = self.accent_reason(a) {
            self.hint = format!("{a:?} is unavailable: {r}");
            return Err(r);
        }
        self.hint.clear();
        self.store.settings.appearance.accent = a;
        self.persist();
        Ok(())
    }

    pub fn calendar_color(&self, id: CalendarId) -> Option<CalendarColor> {
        let c = &self.store.settings.appearance.calendar_colors;
        calendar_slots(c).get(usize::from(id)).copied()
    }

    pub fn calendar_color_reason(&self, id: CalendarId, c: CalendarColor) -> Option<String> {
        slot_unavailable_reason(&self.store.settings.appearance, id, c)
    }

    pub fn calendar_color_name(&self, id: CalendarId, c: CalendarColor) -> String {
        match self.calendar_color_reason(id, c) {
            Some(r) => format!("{c:?}, unavailable: {r}"),
            None if self.calendar_color(id) == Some(c) => format!("{c:?}, selected"),
            None => format!("{c:?}"),
        }
    }

    pub fn set_calendar_color(&mut self, id: CalendarId, c: CalendarColor) -> Result<(), String> {
        let Some(&name) = CALENDAR_NAMES.get(usize::from(id)) else {
            return Err(format!("no calendar {id}"));
        };
        if let Some(r) = self.calendar_color_reason(id, c) {
            self.hint = format!("{c:?} is unavailable for {name}: {r}");
            return Err(r);
        }
        self.hint.clear();
        let cc = &mut self.store.settings.appearance.calendar_colors;
        match id {
            WORK => cc.work = c,
            PERSONAL => cc.personal = c,
            _ => cc.side = c, // SIDE: the length check above leaves no other id
        }
        self.persist();
        Ok(())
    }

    pub fn reset_appearance(&mut self) {
        self.store.settings.reset_appearance();
        let saved = self.persist();
        self.finish_reset(saved);
    }

    /// The status line after a reset: the hint matches how the save ended.
    pub fn finish_reset(&mut self, saved: Save) {
        self.hint = match saved {
            Save::Durable => "Appearance reset to defaults",
            Save::NotDurable => "Appearance reset and saved, but it may be lost if the computer loses power",
            Save::Failed => "Appearance reset, but not saved",
        }
        .into();
    }

    // --- navigation ---

    /// The view unit changes, so what is shown becomes the new base. `want_day` is kept: a 31st
    /// shown as the 28th is still wanted when months are stepped again.
    pub fn set_view(&mut self, v: View) {
        self.base = self.anchor;
        self.steps = 0;
        self.view = v;
        self.reset_year_cursor();
    }

    fn focus_anchor(&mut self, d: NaiveDate) {
        self.anchor = d;
        self.base = d;
        self.steps = 0;
        self.want_day = d.day();
        self.mini = month_start(d);
        self.reset_year_cursor();
    }

    /// Step from the stored base, so 1/31 -> next -> previous is 1/31 again.
    pub fn step(&mut self, dir: i32) {
        self.steps += dir;
        let n = self.steps;
        self.anchor = match self.view {
            View::Day => self.base + Duration::days(n.into()),
            View::Week => self.base + Duration::days(7 * i64::from(n)),
            View::Month => self.months_from_base(n),
            View::Year => self.months_from_base(12 * n),
        };
        if matches!(self.view, View::Day | View::Week) {
            self.want_day = self.anchor.day();
        }
        self.mini = month_start(self.anchor);
        self.reset_year_cursor();
    }

    /// `n` months from the base month, on `want_day` or the month's last day if it is shorter.
    fn months_from_base(&self, n: i32) -> NaiveDate {
        let first = month_start(add_months(month_start(self.base), n));
        let last = month_start(add_months(first, 1)) - Duration::days(1);
        first.with_day(self.want_day.min(last.day())).expect("day within the month")
    }

    pub fn go_today(&mut self) {
        self.focus_anchor(self.today);
    }

    /// "Previous month", "Next week", ... for the current view.
    pub fn step_name(&self, next: bool) -> String {
        format!("{} {}", if next { "Next" } else { "Previous" }, self.view.name().to_lowercase())
    }

    /// Month in `text-1` and year in `text-3`; the Year view has the year alone (no month).
    pub fn title(&self) -> (Option<String>, String) {
        if self.view == View::Year {
            return (None, self.anchor.year().to_string());
        }
        let (month, year) = if matches!(self.view, View::Day | View::Week) { self.week_title() } else { (self.anchor.format("%B").to_string(), self.anchor.year().to_string()) };
        (Some(month), year)
    }

    pub fn pick_day(&mut self, d: NaiveDate) {
        self.focus_anchor(d);
    }

    // --- sidebar ---

    pub fn mini_title(&self) -> String {
        self.mini.format("%B %Y").to_string()
    }

    pub fn mini_step(&mut self, dir: i32) {
        self.mini = month_start(add_months(self.mini, dir));
    }

    pub fn week_start(&self) -> Weekday {
        self.store.settings.week_start
    }

    /// Weekday letters in week order.
    pub fn mini_weekdays(&self) -> Vec<String> {
        let mut d = self.week_start();
        (0..7)
            .map(|_| {
                let l = d.to_string().chars().next().unwrap_or('?').to_string();
                d = d.succ();
                l
            })
            .collect()
    }

    pub fn mini_days(&self) -> Vec<MiniDay> {
        month_grid_range(self.mini.year(), self.mini.month(), self.week_start())
            .days()
            .map(|date| MiniDay {
                date,
                name: date.format("%B %-d").to_string(),
                is_today: date == self.today,
                selected: date == self.anchor,
                out_of_month: date.month() != self.mini.month(),
            })
            .collect()
    }

    /// First visible timed event that has not started yet.
    pub fn next_up(&self) -> Option<NextUp> {
        let now = self.today.and_time(self.now);
        let hidden = |c: CalendarId| self.store.calendars.iter().any(|k| k.id == c && !k.visible);
        let e = self
            .store
            .events
            .iter()
            .filter(|e| !e.all_day && e.start >= now && !hidden(e.calendar))
            .min_by_key(|e| e.start)?;
        let mins = (e.start - now).num_minutes();
        let relative = if e.start.date() == self.today {
            match mins {
                0 => "now".into(),
                1..=59 => format!("in {mins} min"),
                _ => format!("in {} h", mins / 60),
            }
        } else if e.start.date() == self.today + Duration::days(1) {
            "Tomorrow".into()
        } else {
            e.start.format("%b %-d").to_string()
        };
        Some(NextUp {
            id: e.id,
            title: e.title.clone(),
            relative,
            clock: crate::domain::dates::format_time(e.start.time(), self.store.settings.time_format.hour24()),
        })
    }
}
