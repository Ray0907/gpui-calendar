use super::*;

#[test]
fn defaults_match_design() {
    let s = Store::default();
    assert_eq!(s.settings.week_start, Weekday::Mon);
    assert_eq!(s.settings.time_format, TimeFormat::H24);
    let a = &s.settings.appearance;
    assert_eq!((a.mode, a.accent, a.neutral), (Mode::System, Accent::Red, Neutral::Cool));
    assert_eq!(
        (a.calendar_colors.work, a.calendar_colors.personal, a.calendar_colors.side),
        (CalendarColor::Blue, CalendarColor::Green, CalendarColor::Amber)
    );
    let names: Vec<_> = s.calendars.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Work", "Personal", "Side project"]);
    assert!(s.calendars.iter().all(|c| c.visible));
    assert!(!s.events.is_empty());
    assert!(s.events.iter().all(|e| (e.calendar as usize) < s.calendars.len()));
}

#[test]
fn seed_event_ids_unique() {
    let s = Store::default();
    let mut ids: Vec<_> = s.events.iter().map(|e| e.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), s.events.len());
}

#[test]
fn missing_file_loads_defaults_without_bak() {
    let t = Tmp::new();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert!(!bak(&t.file()).exists());
}

#[test]
fn roundtrip() {
    let t = Tmp::new();
    let s = custom();
    s.save(&t.file()).unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), s);
    assert!(!bak(&t.file()).exists());
}

#[test]
fn save_creates_parent_dirs_and_leaves_no_temp() {
    let t = Tmp::new();
    let p = t.0.join("a/b/calendar.json");
    Store::default().save(&p).unwrap();
    assert!(p.exists());
    let names: Vec<_> = fs::read_dir(p.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names.len(), 1);
}

#[test]
fn saved_file_is_versioned_with_documented_shape() {
    let t = Tmp::new();
    Store::default().save(&t.file()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(t.file()).unwrap()).unwrap();
    assert_eq!(v["version"], 1);
    assert!(v["calendars"].is_array() && v["events"].is_array());
    let s = &v["settings"];
    assert_eq!(s["time_format"], "24h");
    assert_eq!(s["appearance"]["mode"], "system");
    assert_eq!(s["appearance"]["accent"], "Red");
    assert_eq!(s["appearance"]["neutral"], "Cool");
    assert_eq!(s["appearance"]["calendar_colors"]["work"], "Blue");
}

#[test]
fn empty_events_stay_empty() {
    let t = Tmp::new();
    let mut s = Store::default();
    s.events.clear();
    s.save(&t.file()).unwrap();
    assert!(Store::load(&t.file()).unwrap().events.is_empty());
}

#[test]
fn tmp_and_bak_names_append_to_the_file_name() {
    let t = Tmp::new();
    for name in ["data", "data.bak", "a.b.c"] {
        let p = t.0.join(name);
        fs::write(&p, "junk").unwrap();
        Store::load(&p).unwrap();
        assert_eq!(fs::read_to_string(t.0.join(format!("{name}.bak"))).unwrap(), "junk", "{name}");
        custom().save(&p).unwrap();
        assert!(p.exists());
    }
}

#[test]
fn save_leaves_no_temp_files() {
    let t = Tmp::new();
    custom().save(&t.file()).unwrap();
    custom().save(&t.file()).unwrap();
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1);
}

#[test]
fn concurrent_saves_use_distinct_temp_names() {
    let t = Tmp::new();
    assert_ne!(temp_path(&t.file()), temp_path(&t.file()));
}

#[test]
fn failed_save_cleans_temp_and_leaves_target() {
    let t = Tmp::new();
    fs::create_dir(t.file()).unwrap(); // rename of a file over a directory fails
    assert!(Store::default().save(&t.file()).is_err());
    assert!(t.file().is_dir());
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn failed_write_leaves_old_file_intact() {
    use std::os::unix::fs::PermissionsExt;
    let t = Tmp::new();
    let old = custom();
    old.save(&t.file()).unwrap();
    fs::set_permissions(&t.0, fs::Permissions::from_mode(0o555)).unwrap();
    let res = Store::default().save(&t.file());
    fs::set_permissions(&t.0, fs::Permissions::from_mode(0o755)).unwrap();
    if res.is_ok() {
        return; // running as a user that ignores directory permissions (root)
    }
    assert_eq!(Store::load(&t.file()).unwrap(), old);
}

#[test]
fn reset_appearance_changes_only_appearance() {
    let mut s = custom();
    assert_ne!(s.settings.appearance, Appearance::default());
    s.settings.reset_appearance();
    assert_eq!(s.settings.appearance, Appearance::default());
    assert_eq!((s.settings.week_start, s.settings.time_format), (Weekday::Sun, TimeFormat::H12));
}

#[test]
fn sync_failure_after_rename_is_saved_but_not_durable() {
    let t = Tmp::new();
    let s = custom();
    let got = s.save_with(&t.file(), |_| Err(io::Error::other("fsync failed"))).unwrap();
    assert!(matches!(got, Saved::NotDurable(ref e) if e.to_string() == "fsync failed"));
    assert_eq!(Store::load(&t.file()).unwrap(), s, "the new file is in place");
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1, "no temp left");
}

#[test]
fn successful_save_is_durable_and_rename_failure_is_still_an_error() {
    let t = Tmp::new();
    assert!(matches!(Store::default().save(&t.file()), Ok(Saved::Durable)));
    let t = Tmp::new();
    fs::create_dir(t.file()).unwrap();
    assert!(Store::default().save_with(&t.file(), |_| Ok(())).is_err());
}
