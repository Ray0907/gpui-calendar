use super::*;

#[test]
fn bad_event_dropped_good_kept_and_original_backed_up() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["events"].as_array_mut().unwrap().insert(0, json!({"id": "nope"}));
    write_json(&t, &v);
    assert_eq!(Store::load(&t.file()).unwrap(), custom());
    let b = baks(&t.0);
    assert_eq!(b.len(), 1);
    assert_eq!(fs::read_to_string(&b[0]).unwrap(), v.to_string());
    assert!(t.file().exists(), "salvaged file stays in place");
}

#[test]
fn clean_file_makes_no_backup() {
    let t = Tmp::new();
    custom().save(&t.file()).unwrap();
    Store::load(&t.file()).unwrap();
    assert!(baks(&t.0).is_empty());
}

#[test]
fn events_of_wrong_type_become_empty_not_seed_and_are_backed_up() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["events"] = "oops".into();
    write_json(&t, &v);
    assert!(Store::load(&t.file()).unwrap().events.is_empty());
    assert_eq!(baks(&t.0).len(), 1);
}

#[test]
fn missing_events_key_uses_seed_without_backup() {
    let t = Tmp::new();
    write_json(&t, &json!({"version": 1}));
    assert_eq!(Store::load(&t.file()).unwrap().events, seed_events());
    assert!(baks(&t.0).is_empty());
}

#[test]
fn existing_bak_is_never_overwritten() {
    let t = Tmp::new();
    for text in ["one", "two", "three"] {
        fs::write(t.file(), text).unwrap();
        Store::load(&t.file()).unwrap();
    }
    let contents: Vec<_> = baks(&t.0).iter().map(|p| fs::read_to_string(p).unwrap()).collect();
    assert_eq!(contents.len(), 3);
    for text in ["one", "two", "three"] {
        assert!(contents.iter().any(|c| c == text), "{text} lost");
    }
}

#[test]
fn salvage_backup_does_not_overwrite_earlier_bak() {
    let t = Tmp::new();
    fs::write(t.file(), "garbage").unwrap();
    Store::load(&t.file()).unwrap(); // moves to .bak
    let mut v = serde_json::to_value(custom()).unwrap();
    v["events"] = json!(7);
    write_json(&t, &v);
    Store::load(&t.file()).unwrap();
    let contents: Vec<_> = baks(&t.0).iter().map(|p| fs::read_to_string(p).unwrap()).collect();
    assert_eq!(contents.len(), 2);
    assert!(contents.iter().any(|c| c == "garbage"));
}

#[test]
fn unreadable_path_is_an_error_and_is_not_renamed() {
    let t = Tmp::new();
    fs::create_dir(t.file()).unwrap(); // read_to_string fails with a non-InvalidData error
    assert!(Store::load(&t.file()).is_err());
    assert!(t.file().is_dir());
    assert!(baks(&t.0).is_empty());
}

#[test]
fn non_utf8_file_is_corrupt_and_kept() {
    let t = Tmp::new();
    fs::write(t.file(), [0xff, 0xfe, 0x00, 0x7b]).unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert_eq!(fs::read(bak(&t.file())).unwrap(), [0xff, 0xfe, 0x00, 0x7b]);
}

#[test]
fn identical_backup_is_not_repeated() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["events"] = json!(7);
    for _ in 0..3 {
        write_json(&t, &v);
        Store::load(&t.file()).unwrap();
    }
    assert_eq!(baks(&t.0).len(), 1, "lossy salvage");
    let t = Tmp::new();
    for _ in 0..3 {
        fs::write(t.file(), "garbage").unwrap();
        Store::load(&t.file()).unwrap();
    }
    assert_eq!(baks(&t.0).len(), 1, "quarantine");
    fs::write(t.file(), "different garbage").unwrap();
    Store::load(&t.file()).unwrap();
    assert_eq!(baks(&t.0).len(), 2, "different content still gets its own backup");
}

fn tmp_aged(t: &Tmp, name: &str, age_secs: u64) -> PathBuf {
    let p = t.0.join(name);
    let f = fs::File::create(&p).unwrap();
    f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(age_secs)).unwrap();
    p
}

#[test]
fn load_removes_stale_temp_files_only() {
    let t = Tmp::new();
    custom().save(&t.file()).unwrap();
    let stale = tmp_aged(&t, "calendar.json.4242.0.tmp", 2 * 3600);
    let fresh = tmp_aged(&t, "calendar.json.4242.1.tmp", 5); // may belong to a save in flight
    let other = tmp_aged(&t, "notes.json.4242.0.tmp", 2 * 3600);
    assert_eq!(Store::load(&t.file()).unwrap(), custom());
    assert!(!stale.exists());
    assert!(fresh.exists());
    assert!(other.exists(), "only temp files of this store file");
}

#[test]
fn stale_temp_files_are_removed_even_when_the_target_is_missing() {
    let t = Tmp::new();
    let stale = tmp_aged(&t, "calendar.json.4242.0.tmp", 2 * 3600);
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert!(!stale.exists());
}

#[test]
fn lone_calendar_entry_with_bad_visible_is_lossy_and_backed_up() {
    for entry in [json!({"id": 1}), json!({"id": 1, "visible": "no"}), json!({"id": 1, "visible": null})] {
        let t = Tmp::new();
        let mut v = serde_json::to_value(custom()).unwrap();
        v["calendars"] = json!([entry]); // no duplicate, so only the bad `visible` can flag the load
        write_json(&t, &v);
        assert!(Store::load(&t.file()).unwrap().calendars[1].visible, "{entry}");
        let b = baks(&t.0);
        assert_eq!(b.len(), 1, "{entry}");
        assert_eq!(fs::read_to_string(&b[0]).unwrap(), v.to_string(), "{entry}");
    }
}

#[cfg(unix)]
#[test]
fn permission_denied_read_is_an_error_without_rename_or_backup() {
    use std::os::unix::fs::PermissionsExt;
    let t = Tmp::new();
    let text = serde_json::to_string(&custom()).unwrap();
    fs::write(t.file(), &text).unwrap();
    fs::set_permissions(t.file(), fs::Permissions::from_mode(0o000)).unwrap();
    let res = Store::load(&t.file());
    fs::set_permissions(t.file(), fs::Permissions::from_mode(0o644)).unwrap();
    if res.is_ok() {
        return; // running as a user that ignores file permissions (root)
    }
    assert_eq!(res.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(fs::read_to_string(t.file()).unwrap(), text, "file not renamed or touched");
    assert!(baks(&t.0).is_empty());
}

#[test]
fn directory_read_error_is_not_invalid_data_and_nothing_is_created() {
    let t = Tmp::new();
    fs::create_dir(t.file()).unwrap();
    let kind = Store::load(&t.file()).unwrap_err().kind();
    assert!(kind != io::ErrorKind::InvalidData && kind != io::ErrorKind::NotFound, "{kind:?}");
    assert!(t.file().is_dir());
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1);
}
