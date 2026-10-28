//! Versioned JSON persistence. No UI-crate import.
//!
//! Load rules:
//! - missing file: defaults.
//! - unreadable for any reason other than invalid bytes (permissions, I/O): `Err`, file untouched.
//! - invalid UTF-8, invalid JSON, not an object, or another version: the file is moved to
//!   `<name>.bak` and defaults are returned.
//! - otherwise every field falls back to its default on its own. If anything was dropped or
//!   repaired, the original file is first copied to `<name>.bak`, so the next save cannot lose it.
//!   Backups never overwrite: later ones are `<name>.bak.1`, `.bak.2`, ...
//! - calendars: entries are `{id, visible}`. A bad or missing `visible` keeps the default but the
//!   id still counts (a later duplicate cannot win). Entries without a usable id, ids outside the
//!   fixed set and repeated ids (first wins) are ignored.
//! - events: `id`, `calendar` (must exist) and `start` are required, else the event is dropped.
//!   Bad `title` / `all_day` become `""` / `false`. An `end` that is missing, unparsable or not
//!   after `start` is repaired to start + 15 minutes (all-day: + 1 day). A repeated `id` keeps
//!   the first event; later ones get fresh ids above the largest id in the file, in file order.
//!   A missing `events` key gives the seed events; a wrong-typed one gives no events.
//!
//! - unknown keys (top level, calendar entries, events, settings) are not kept in memory, so a
//!   file that has any is backed up before the next save, like any other drop.
//! - backups whose bytes equal an existing `.bak*` are not repeated.
//! - load removes `<name>.*.tmp` files older than an hour (left by a crashed save).
//!
//! Save writes a uniquely named temp file, fsyncs it, renames it over the target and fsyncs the
//! parent directory (unix). `Saved::NotDurable` means the rename happened but the directory
//! fsync failed; plain `Err` means the old file is untouched.

pub mod settings;

use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::event::{CalendarId, Event};
use crate::domain::layout::SNAP_MINUTES;
pub use settings::*;
use settings::{flag_unknown, get};

pub const VERSION: u32 = 1;
const APP_DIR: &str = "gpui-calendar";
const FILE_NAME: &str = "calendar.json";

pub const WORK: CalendarId = 0;
pub const PERSONAL: CalendarId = 1;
pub const SIDE: CalendarId = 2;

/// The fixed calendar set. Name and shape are fixed; only `visible` is user state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    pub id: CalendarId,
    pub name: String,
    pub visible: bool,
}

/// Default calendar names in `CalendarId` order (`WORK`, `PERSONAL`, `SIDE`); the theme rules quote them.
pub const CALENDAR_NAMES: [&str; 3] = ["Work", "Personal", "Side project"];

fn default_calendars() -> Vec<Calendar> {
    CALENDAR_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| Calendar { id: i as CalendarId, name: (*n).into(), visible: true })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Store {
    pub version: u32,
    pub calendars: Vec<Calendar>,
    pub events: Vec<Event>,
    pub settings: Settings,
}

impl Default for Store {
    fn default() -> Self {
        Self { version: VERSION, calendars: default_calendars(), events: seed_events(), settings: Settings::default() }
    }
}

/// Invented sample events on fixed dates (no clock), so a first run is not empty.
pub fn seed_events() -> Vec<Event> {
    let d = |m, day| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
    let t = |m, day, h, mi| d(m, day).and_hms_opt(h, mi, 0).unwrap();
    vec![
        Event::timed(1, "Sprint planning", WORK, t(10, 26, 10, 0), t(10, 26, 11, 0)),
        Event::timed(2, "Design review", WORK, t(10, 27, 14, 0), t(10, 27, 15, 30)),
        Event::timed(3, "Release checklist", WORK, t(10, 27, 14, 30), t(10, 27, 15, 0)),
        Event::timed(4, "Standup", WORK, t(10, 28, 9, 30), t(10, 28, 9, 45)),
        Event::timed(5, "Piano lesson", PERSONAL, t(10, 28, 18, 0), t(10, 28, 19, 0)),
        Event::timed(6, "Dentist", PERSONAL, t(10, 29, 8, 30), t(10, 29, 9, 15)),
        Event::timed(7, "Weekend project: garden shed", SIDE, t(10, 31, 10, 0), t(10, 31, 13, 0)),
        Event::timed(8, "Late deploy window", WORK, t(10, 30, 22, 0), t(10, 31, 1, 0)),
        Event::all_day(9, "Conference", WORK, d(11, 3), d(11, 5)),
        Event::all_day(10, "Hiking trip", PERSONAL, d(11, 7), d(11, 8)),
        Event::all_day(11, "Open-source release", SIDE, d(11, 12), d(11, 12)),
        Event::timed(12, "Book club", PERSONAL, t(11, 10, 19, 0), t(11, 10, 20, 30)),
        Event::timed(13, "Prototype sync", SIDE, d(11, 4).and_hms_opt(20, 0, 0).unwrap(), d(11, 4).and_hms_opt(20, 0, 0).unwrap() + Duration::minutes(45)),
    ]
}

/// `<file name><suffix>` next to `path` (never `with_extension`, which rewrites the extension).
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Unique temp file used by `save`, a sibling of `path` so the rename stays on one filesystem.
pub fn temp_path(path: &Path) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    with_suffix(path, &format!(".{}.{}.tmp", std::process::id(), N.fetch_add(1, Ordering::Relaxed)))
}

/// Copy `path` to the first free `<path>.bak`, `.bak.1`, ... `create_new` makes overwriting
/// impossible. If an existing backup already holds the same bytes, nothing is written.
fn backup(path: &Path) -> io::Result<()> {
    let data = fs::read(path)?;
    let mut n = 0u32;
    loop {
        let dst = if n == 0 { with_suffix(path, ".bak") } else { with_suffix(path, &format!(".bak.{n}")) };
        match fs::read(&dst) {
            Ok(old) if old == data => return Ok(()),
            Ok(_) => n += 1,
            Err(e) if e.kind() == io::ErrorKind::NotFound => match fs::OpenOptions::new().write(true).create_new(true).open(&dst) {
                Ok(mut f) => {
                    f.write_all(&data)?;
                    return f.sync_all();
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {} // lost a race: look at this slot again
                Err(e) => return Err(e),
            },
            Err(e) => return Err(e),
        }
    }
}

/// A temp file this old cannot belong to a save in flight.
const STALE_TMP_AGE: std::time::Duration = std::time::Duration::from_secs(3600);

/// Best-effort removal of `<name>.*.tmp` files a crashed save left next to `path`.
fn remove_stale_temps(path: &Path) {
    let (Some(name), Some(dir)) = (path.file_name(), path.parent()) else { return };
    let prefix = format!("{}.", name.to_string_lossy());
    let dir = if dir.as_os_str().is_empty() { Path::new(".") } else { dir };
    let Ok(entries) = fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let file = e.file_name().to_string_lossy().into_owned();
        let stale = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age >= STALE_TMP_AGE);
        if file.starts_with(&prefix) && file.ends_with(".tmp") && stale {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// Keep the file as a backup and remove it from `path`.
fn quarantine(path: &Path) -> io::Result<()> {
    backup(path)?;
    fs::remove_file(path)
}

/// OS config location, e.g. `~/Library/Application Support/gpui-calendar/calendar.json`.
/// `None` if the home directory cannot be found.
pub fn default_path() -> Option<PathBuf> {
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        var("HOME")?.join("Library/Application Support")
    } else if cfg!(windows) {
        var("APPDATA")?
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|h| h.join(".config")))?
    };
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// What `Store::load_noted` did to the file besides reading it; either way a `.bak` copy of the original exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadNote {
    /// Not usable (corrupt, invalid text or another version): moved aside, defaults returned.
    Quarantined,
    /// Usable, but some fields were dropped or repaired.
    Repaired,
}

impl Store {
    /// See the module docs for what is salvaged. `Err` only for read errors other than
    /// missing or invalid bytes, and for failing to write a backup.
    pub fn load(path: &Path) -> io::Result<Store> {
        Self::load_noted(path).map(|(store, _)| store)
    }

    /// Like `load`, plus whether the original file was kept as a backup on the way.
    pub fn load_noted(path: &Path) -> io::Result<(Store, Option<LoadNote>)> {
        remove_stale_temps(path);
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((Store::default(), None)),
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                quarantine(path)?;
                return Ok((Store::default(), Some(LoadNote::Quarantined)));
            }
            Err(e) => return Err(e),
        };
        match serde_json::from_str::<Value>(&text).ok().and_then(|v| Store::from_json(&v)) {
            Some((store, lossy)) => {
                if lossy {
                    backup(path)?;
                }
                Ok((store, lossy.then_some(LoadNote::Repaired)))
            }
            None => {
                quarantine(path)?;
                Ok((Store::default(), Some(LoadNote::Quarantined)))
            }
        }
    }

    /// The store and whether anything was dropped or repaired; `None` if `v` is not an object
    /// of the current version.
    fn from_json(v: &Value) -> Option<(Store, bool)> {
        let obj = v.as_object()?;
        if obj.get("version").and_then(Value::as_u64) != Some(VERSION as u64) {
            return None;
        }
        let mut lossy = false;
        flag_unknown(Some(obj), &["version", "calendars", "events", "settings"], &mut lossy);
        let mut calendars = default_calendars();
        match obj.get("calendars") {
            None => {}
            Some(Value::Array(list)) => {
                let mut seen = HashSet::new();
                for entry in list {
                    let obj = entry.as_object();
                    flag_unknown(obj, &["id", "name", "visible"], &mut lossy);
                    // The id is what makes an entry usable; a bad `visible` keeps the default
                    // but still takes the id, so a later duplicate cannot win.
                    let Some(id) = get::<CalendarId>(obj, "id").filter(|id| seen.insert(*id)) else {
                        lossy = true;
                        continue;
                    };
                    let visible = get::<bool>(obj, "visible");
                    lossy |= visible.is_none();
                    match calendars.iter_mut().find(|c| c.id == id) {
                        Some(c) => c.visible = visible.unwrap_or(c.visible),
                        None => lossy = true,
                    }
                }
            }
            Some(_) => lossy = true,
        }
        let events = match obj.get("events") {
            None => seed_events(),
            Some(Value::Array(list)) => {
                let mut events = Vec::new();
                for e in list {
                    match parse_event(e, &calendars) {
                        Some((e, repaired)) => {
                            lossy |= repaired;
                            events.push(e);
                        }
                        None => lossy = true,
                    }
                }
                lossy |= dedupe_ids(&mut events);
                events
            }
            Some(_) => {
                lossy = true;
                Vec::new()
            }
        };
        let (settings, repaired) = Settings::from_json(obj.get("settings"));
        Some((Store { version: VERSION, calendars, events, settings }, lossy | repaired))
    }

    /// Write temp file, flush to disk, rename over `path`, fsync the parent directory.
    /// `Err`: nothing was replaced, the old file is untouched. `Ok(Saved::NotDurable)`: the new
    /// file is in place but the directory fsync failed, so it may not survive a power cut.
    pub fn save(&self, path: &Path) -> io::Result<Saved> {
        self.save_with(path, sync_parent)
    }

    fn save_with(&self, path: &Path, sync_dir: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<Saved> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        let tmp = temp_path(path);
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let written = fs::File::create(&tmp).and_then(|mut f| {
            f.write_all(&json)?;
            f.sync_all()
        });
        match written.and_then(|_| fs::rename(&tmp, path)) {
            Ok(()) => Ok(match sync_dir(path) {
                Ok(()) => Saved::Durable,
                Err(e) => Saved::NotDurable(e),
            }),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    }
}

/// Outcome of a `save` that replaced the file.
#[derive(Debug)]
pub enum Saved {
    Durable,
    /// The rename happened but fsyncing the parent directory failed.
    NotDurable(io::Error),
}

/// Make the rename durable: fsync the directory entry (unix only; other platforms have no equivalent).
fn sync_parent(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::File::open(dir)?.sync_all()?;
    }
    let _ = path;
    Ok(())
}

/// One event, and whether any field had to be defaulted or repaired. `None` if a required field is bad.
fn parse_event(v: &Value, calendars: &[Calendar]) -> Option<(Event, bool)> {
    let obj = v.as_object();
    let mut unknown = false;
    flag_unknown(obj, &["id", "title", "calendar", "start", "end", "all_day"], &mut unknown);
    let id: u64 = get(obj, "id")?;
    let calendar: CalendarId = get(obj, "calendar").filter(|c| calendars.iter().any(|k| k.id == *c))?;
    let start: NaiveDateTime = get(obj, "start")?;
    let title = get::<String>(obj, "title");
    let all_day = get::<bool>(obj, "all_day");
    let end = get::<NaiveDateTime>(obj, "end").filter(|e| *e > start);
    let repaired = unknown || title.is_none() || all_day.is_none() || end.is_none();
    let all_day = all_day.unwrap_or(false);
    let end = end.unwrap_or(start + if all_day { Duration::days(1) } else { Duration::minutes(SNAP_MINUTES as i64) });
    Some((Event { id, title: title.unwrap_or_default(), calendar, start, end, all_day }, repaired))
}

/// Give repeated ids fresh ones (first occurrence keeps its id). True if any changed.
fn dedupe_ids(events: &mut [Event]) -> bool {
    let mut next = events.iter().map(|e| e.id).max().unwrap_or(0);
    let mut seen = HashSet::new();
    let mut changed = false;
    for e in events {
        if !seen.insert(e.id) {
            next = next.wrapping_add(1); // ponytail: wraps only at u64::MAX, ignored
            e.id = next;
            seen.insert(next);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests;
