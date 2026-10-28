//! Behavior of the shell model: store writes and persistence, no GPUI.

use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveTime};

use crate::store::settings::{Accent, CalendarColor, Mode, Neutral};
use crate::store::{PERSONAL, SIDE, Saved, Store, WORK};
use crate::ui::model::{Model, Screen};

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gpui-calendar-ui-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir.join("calendar.json")
}

fn model(path: &Path) -> Model {
    let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
    Model::new(Store::default(), Some(path.to_path_buf()), today, NaiveTime::from_hms_opt(17, 35, 0).unwrap(), false)
}

fn saved(path: &Path) -> Store {
    Store::load(path).unwrap()
}

#[test]
fn toggling_a_calendar_updates_visibility_in_the_store_and_on_disk() {
    let p = tmp("toggle");
    let mut m = model(&p);
    assert!(m.store.calendars[WORK as usize].visible);
    m.toggle_calendar(WORK);
    assert!(!m.store.calendars[WORK as usize].visible);
    assert!(!saved(&p).calendars[WORK as usize].visible);
    m.toggle_calendar(WORK);
    assert!(saved(&p).calendars[WORK as usize].visible);
    m.toggle_calendar(99); // unknown id: no change, no panic
    assert!(m.store.calendars.iter().all(|c| c.visible));
}

#[test]
fn changing_accent_applies_to_the_theme_and_persists() {
    let p = tmp("accent");
    let mut m = model(&p);
    let before = m.theme().accent;
    assert_eq!(m.set_accent(Accent::Violet), Ok(()));
    assert_ne!(m.theme().accent, before);
    assert_eq!(saved(&p).settings.appearance.accent, Accent::Violet);
}

#[test]
fn a_disabled_accent_refuses_and_exposes_its_reason() {
    let p = tmp("accent-refuse");
    let mut m = model(&p);
    // Orange hue 48 is within 15 degrees of nothing by default; Blue calendar (265) blocks Indigo (278).
    let reason = m.accent_reason(Accent::Indigo).expect("indigo is too close to Work blue");
    assert!(reason.contains("Work"), "{reason}");
    assert_eq!(m.set_accent(Accent::Indigo), Err(reason.clone()));
    assert_eq!(m.store.settings.appearance.accent, Accent::Red);
    assert_eq!(m.hint, format!("Indigo is unavailable: {reason}"));
    assert!(!p.exists(), "a refused change must not write");
    assert!(m.accent_reason(Accent::Red).is_none(), "the current accent stays available");
    assert_eq!(m.accent_name(Accent::Indigo), format!("Indigo, unavailable: {reason}"));
    assert_eq!(m.accent_name(Accent::Red), "Red, selected");
    assert_eq!(m.accent_name(Accent::Teal), "Teal");
}

#[test]
fn calendar_colors_refuse_the_accent_hue_and_taken_colors() {
    let p = tmp("slots");
    let mut m = model(&p);
    let r = m.set_calendar_color(PERSONAL, CalendarColor::Red).unwrap_err();
    assert_eq!(r, "too close to the accent, which marks today");
    assert_eq!(
        m.calendar_color_name(PERSONAL, CalendarColor::Red),
        "Red, unavailable: too close to the accent, which marks today"
    );
    assert_eq!(m.set_calendar_color(SIDE, CalendarColor::Blue).unwrap_err(), "already used by Work");
    assert_eq!(m.set_calendar_color(SIDE, CalendarColor::Pink), Ok(()));
    assert_eq!(saved(&p).settings.appearance.calendar_colors.side, CalendarColor::Pink);
    assert_eq!(m.store.settings.appearance.calendar_colors.personal, CalendarColor::Green);
}

#[test]
fn reset_restores_every_appearance_default_and_persists() {
    let p = tmp("reset");
    let mut m = model(&p);
    m.set_mode(Mode::Dark);
    m.set_neutral(Neutral::Warm);
    m.set_accent(Accent::Teal).unwrap();
    m.set_calendar_color(SIDE, CalendarColor::Pink).unwrap();
    assert_ne!(saved(&p).settings.appearance, Default::default());
    m.reset_appearance();
    assert_eq!(m.store.settings.appearance, Default::default());
    assert_eq!(saved(&p).settings.appearance, Default::default());
    assert!(m.hint.is_empty() || m.hint.contains("default"));
}

#[test]
fn mode_switch_changes_dark_and_system_follows_the_os() {
    let p = tmp("mode");
    let mut m = model(&p);
    assert!(!m.theme().dark, "System with a light OS is light");
    m.set_mode(Mode::Dark);
    assert!(m.theme().dark);
    assert_eq!(saved(&p).settings.appearance.mode, Mode::Dark);
    m.set_mode(Mode::System);
    m.system_dark = true;
    assert!(m.theme().dark, "System follows the OS live");
    m.set_mode(Mode::Light);
    assert!(!m.theme().dark, "Light ignores the OS");
}

#[test]
fn a_failed_save_is_reported_in_the_status_line() {
    // A file where the parent directory should be makes create_dir_all fail.
    let blocker = tmp("blocked");
    std::fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    std::fs::write(blocker.parent().unwrap().join("sub"), b"x").unwrap();
    let mut m = model(&blocker.parent().unwrap().join("sub").join("calendar.json"));
    m.set_mode(Mode::Dark);
    assert!(m.theme().dark, "the change still applies");
    assert!(m.notice.contains("not saved"), "{}", m.notice);
    // Not only Appearance: a calendar toggle on the calendar screen reports it too.
    m.notice.clear();
    m.toggle_calendar(WORK);
    assert!(m.notice.contains("not saved"), "{}", m.notice);
}

#[test]
fn a_later_successful_save_leaves_the_notice_for_the_user_to_dismiss() {
    let mut m = model(&tmp("recovers"));
    m.notice = "Settings not saved: disk full".into();
    m.set_mode(Mode::Dark);
    assert_eq!(m.notice, "Settings not saved: disk full");
    m.dismiss_notice();
    assert!(m.notice.is_empty());
}

#[test]
fn a_load_error_keeps_the_file_and_disables_saving_with_a_visible_message() {
    let p = tmp("load-error");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, b"{\"keep\": \"me\"}").unwrap();
    let before = std::fs::read(&p).unwrap();
    let mut m = model(&p);
    m.block_saving("permission denied".into());
    assert!(m.notice.contains("permission denied") && m.notice.contains("not saved"), "{}", m.notice);
    m.set_mode(Mode::Dark);
    assert!(m.theme().dark, "the change still applies in memory");
    m.toggle_calendar(WORK);
    m.reset_appearance();
    assert_eq!(std::fs::read(&p).unwrap(), before, "the user's file must be untouched");
    assert!(m.notice.contains("permission denied"), "the message stays until resolved: {}", m.notice);
}

#[test]
fn calendar_colors_reject_ids_outside_the_fixed_set() {
    let p = tmp("bad-id");
    let mut m = model(&p);
    let before = m.store.settings.appearance;
    assert_eq!(m.calendar_color(3), None);
    assert!(m.set_calendar_color(3, CalendarColor::Pink).is_err());
    assert!(m.set_calendar_color(200, CalendarColor::Pink).is_err());
    assert_eq!(m.store.settings.appearance, before, "nothing written for a bad id");
    assert!(!p.exists());
    for id in [WORK, PERSONAL, SIDE] {
        assert!(m.calendar_color(id).is_some());
    }
}

#[test]
fn appearance_screen_toggles_from_the_account_menu() {
    let mut m = model(&tmp("screen"));
    assert_eq!(m.screen, Screen::Calendar);
    m.toggle_appearance();
    assert_eq!(m.screen, Screen::Appearance);
    m.toggle_appearance();
    assert_eq!(m.screen, Screen::Calendar);
}

fn dir_blocked(name: &str) -> (PathBuf, PathBuf) {
    let p = tmp(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let file = p.parent().unwrap().join("sub");
    std::fs::write(&file, b"x").unwrap();
    (file.join("calendar.json"), file)
}

#[test]
fn saving_without_a_path_says_so() {
    let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
    let mut m = Model::new(Store::default(), None, today, NaiveTime::from_hms_opt(9, 0, 0).unwrap(), false);
    assert!(m.notice.is_empty());
    m.set_mode(Mode::Dark);
    assert!(m.notice == crate::ui::model::NO_PATH, "{}", m.notice);
    assert!(m.theme().dark, "the change still applies on screen");
}

#[test]
fn a_save_that_is_not_durable_is_not_a_clean_save() {
    let mut m = model(&tmp("nodur"));
    assert!(!m.record_save(Ok(Saved::NotDurable(std::io::Error::other("fsync failed")))));
    assert!(m.notice.contains("fsync failed") && m.notice.contains("power"), "{}", m.notice);
    let mut m = model(&tmp("dur"));
    assert!(m.record_save(Ok(Saved::Durable)) && m.notice.is_empty());
}

#[test]
fn a_later_success_keeps_an_earlier_failure_until_it_is_dismissed() {
    let (path, file) = dir_blocked("sticky-model");
    let mut m = model(&path);
    m.toggle_calendar(WORK);
    let first = m.notice.clone();
    assert!(first.contains("not saved"));
    std::fs::remove_file(&file).unwrap();
    m.toggle_calendar(WORK);
    assert!(saved(&path).calendars[WORK as usize].visible, "the second save worked");
    assert_eq!(m.notice, first, "success does not clear the failure");
    m.dismiss_notice();
    assert!(m.notice.is_empty());
    m.toggle_calendar(WORK);
    assert!(m.notice.is_empty(), "a success after a dismiss stays quiet");
}

#[test]
fn reset_hint_never_claims_a_save_that_failed() {
    let (path, file) = dir_blocked("reset-hint");
    let mut m = model(&path);
    m.reset_appearance();
    assert!(!m.notice.is_empty());
    assert!(m.hint.contains("not saved"), "{}", m.hint);
    std::fs::remove_file(&file).unwrap();
    m.reset_appearance();
    assert_eq!(m.hint, "Appearance reset to defaults");
}

#[test]
fn a_blocked_store_stays_blocked_after_a_dismiss_and_retry_unblocks_it() {
    let p = tmp("retry-model");
    std::fs::create_dir_all(&p).unwrap();
    let mut m = model(&p);
    m.block_saving("is a directory".into());
    assert!(m.can_retry());
    m.dismiss_notice();
    m.toggle_calendar(WORK);
    assert!(m.notice.contains("is a directory"), "the blocked state speaks again");
    m.retry_load();
    assert!(m.notice.to_lowercase().contains("is a directory") && m.can_retry(), "still unreadable: still blocked");
    std::fs::remove_dir_all(&p).unwrap();
    let mut on_disk = Store::default();
    on_disk.calendars[WORK as usize].visible = false;
    on_disk.save(&p).unwrap();
    m.retry_load();
    assert!(m.notice.is_empty() && !m.can_retry());
    assert!(!m.store.calendars[WORK as usize].visible, "the file's contents are now loaded");
    m.toggle_calendar(PERSONAL);
    assert!(!saved(&p).calendars[PERSONAL as usize].visible);
}

#[test]
fn a_second_failure_does_not_replace_the_first_unresolved_one() {
    let mut m = model(&tmp("two-failures"));
    m.record_save(Err(std::io::Error::other("disk full")));
    m.record_save(Err(std::io::Error::other("read-only")));
    assert!(m.notice.contains("disk full") && !m.notice.contains("read-only"), "{}", m.notice);
}
