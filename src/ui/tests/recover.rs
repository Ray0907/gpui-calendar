//! Retrying a blocked load and resetting appearance: what is kept, what is said.

use std::io;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveTime};

use crate::store::settings::{Accent, Mode};
use crate::store::{PERSONAL, Saved, Store, WORK};
use crate::ui::model::{Model, Save};

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gpui-calendar-recover-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("calendar.json")
}

fn model(path: &Path) -> Model {
    let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
    Model::new(Store::default(), Some(path.to_path_buf()), today, NaiveTime::from_hms_opt(17, 35, 0).unwrap(), false)
}

/// A readable file on disk: Teal accent, Personal hidden.
fn disk(path: &Path) -> Store {
    let mut s = Store::default();
    s.settings.appearance.accent = Accent::Teal;
    s.calendars[usize::from(PERSONAL)].visible = false;
    s.save(path).unwrap();
    s
}

#[test]
fn retry_keeps_changes_made_while_saving_was_off_and_clears_only_its_own_notice() {
    let p = tmp("merge");
    disk(&p);
    let mut m = model(&p);
    m.block_saving("unreadable".into());
    m.set_mode(Mode::Dark); // in memory only
    m.toggle_calendar(WORK);
    m.dismiss_notice();
    m.warn("Settings not saved: disk full".into()); // a different problem
    m.retry_load();
    assert!(!m.can_retry());
    assert_eq!(m.notice, "Settings not saved: disk full", "a notice that is not the load notice stays");
    let a = m.store.settings.appearance;
    assert_eq!((a.mode, a.accent), (Mode::Dark, Accent::Teal), "my change kept, the file's other fields taken");
    assert!(!m.store.calendars[usize::from(WORK)].visible && !m.store.calendars[usize::from(PERSONAL)].visible);
    assert_eq!(Store::load(&p).unwrap(), m.store, "the merge is on disk");
}

#[test]
fn retry_says_it_merged_and_without_local_changes_just_takes_the_file() {
    let p = tmp("say");
    let on_disk = disk(&p);
    let mut m = model(&p);
    m.block_saving("unreadable".into());
    m.retry_load();
    assert_eq!(m.store, on_disk);
    assert!(m.notice.is_empty(), "nothing to merge, nothing to say: {}", m.notice);

    let mut m = model(&p);
    m.block_saving("unreadable".into());
    m.set_mode(Mode::Dark);
    m.retry_load();
    assert!(m.notice.contains("kept"), "{}", m.notice);
    assert_eq!(m.store.settings.appearance.mode, Mode::Dark);
}

#[test]
fn reset_says_so_when_the_save_may_not_be_durable() {
    let mut m = model(&tmp("reset-nd"));
    let s = m.record(Ok(Saved::NotDurable(io::Error::other("fsync failed"))));
    assert!(matches!(s, Save::NotDurable));
    m.finish_reset(s);
    assert!(m.hint.contains("may be lost"), "{}", m.hint);
    assert!(!m.hint.contains("not saved"), "{}", m.hint);
    let s = m.record(Err(io::Error::other("disk full")));
    m.finish_reset(s);
    assert_eq!(m.hint, "Appearance reset, but not saved");
    m.finish_reset(Save::Durable);
    assert_eq!(m.hint, "Appearance reset to defaults");
}
