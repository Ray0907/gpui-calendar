use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::Weekday;

use super::*;
use serde_json::{Value, json};

/// Unique temp dir, removed on drop.
struct Tmp(PathBuf);

impl Tmp {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!("gpui-calendar-store-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
        fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
    fn file(&self) -> PathBuf {
        self.0.join("calendar.json")
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bak(p: &Path) -> PathBuf {
    p.with_extension("json.bak")
}

/// All backup files next to `calendar.json`, sorted by name.
fn baks(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("calendar.json.bak"))
        .collect();
    v.sort();
    v
}

fn write_json(t: &Tmp, v: &Value) {
    fs::write(t.file(), v.to_string()).unwrap();
}

fn custom() -> Store {
    let mut s = Store::default();
    s.events.push(Event::timed(
        900,
        "Custom",
        1,
        chrono::NaiveDate::from_ymd_opt(2027, 1, 5).unwrap().and_hms_opt(9, 15, 0).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2027, 1, 5).unwrap().and_hms_opt(10, 0, 0).unwrap(),
    ));
    s.calendars[2].visible = false;
    s.settings.week_start = Weekday::Sun;
    s.settings.time_format = TimeFormat::H12;
    s.settings.appearance.mode = Mode::Dark;
    s.settings.appearance.accent = Accent::Violet;
    s.settings.appearance.neutral = Neutral::Warm;
    s.settings.appearance.calendar_colors.side = CalendarColor::Pink;
    s
}

type Mutate = Box<dyn Fn(&mut Value)>;

mod backup;
mod events;
mod guards;
mod load_fallback;
mod save;
