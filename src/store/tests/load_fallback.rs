use super::*;

#[test]
fn corrupt_file_loads_defaults_and_keeps_bak() {
    let t = Tmp::new();
    fs::write(t.file(), "{ not json \u{0}").unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert_eq!(fs::read_to_string(bak(&t.file())).unwrap(), "{ not json \u{0}");
    assert!(!t.file().exists());
}

#[test]
fn non_object_json_is_corrupt() {
    let t = Tmp::new();
    fs::write(t.file(), "[1,2,3]").unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert!(bak(&t.file()).exists());
}

#[test]
fn empty_file_is_corrupt() {
    let t = Tmp::new();
    fs::write(t.file(), "").unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert!(bak(&t.file()).exists());
}

#[test]
fn wrong_version_loads_defaults_and_keeps_bak() {
    let t = Tmp::new();
    let text = r#"{"version": 99, "events": [], "settings": {"week_start": "Sun"}}"#;
    fs::write(t.file(), text).unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert_eq!(fs::read_to_string(bak(&t.file())).unwrap(), text);
}

#[test]
fn missing_version_is_wrong_version() {
    let t = Tmp::new();
    fs::write(t.file(), r#"{"events": []}"#).unwrap();
    assert_eq!(Store::load(&t.file()).unwrap(), Store::default());
    assert!(bak(&t.file()).exists());
}

#[test]
fn unknown_accent_falls_back_but_keeps_other_fields() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["settings"]["appearance"]["accent"] = "Chartreuse".into();
    fs::write(t.file(), v.to_string()).unwrap();
    let got = Store::load(&t.file()).unwrap();
    let mut want = custom();
    want.settings.appearance.accent = Accent::Red;
    assert_eq!(got, want);
    assert_eq!(fs::read_to_string(bak(&t.file())).unwrap(), v.to_string(), "repair must back up the original");
}

#[test]
fn wrong_type_falls_back_per_field() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["settings"]["week_start"] = 7.into();
    v["settings"]["appearance"]["neutral"] = serde_json::json!({"x": 1});
    v["settings"]["appearance"]["calendar_colors"]["work"] = true.into();
    fs::write(t.file(), v.to_string()).unwrap();
    let got = Store::load(&t.file()).unwrap();
    let mut want = custom();
    want.settings.week_start = Weekday::Mon;
    want.settings.appearance.neutral = Neutral::Cool;
    want.settings.appearance.calendar_colors.work = CalendarColor::Blue;
    assert_eq!(got, want);
}

#[test]
fn missing_settings_uses_defaults_and_keeps_events() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v.as_object_mut().unwrap().remove("settings");
    fs::write(t.file(), v.to_string()).unwrap();
    let got = Store::load(&t.file()).unwrap();
    let mut want = custom();
    want.settings = Settings::default();
    assert_eq!(got, want);
    assert!(!bak(&t.file()).exists());
}

#[test]
fn missing_nested_appearance_fields_use_defaults() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["settings"]["appearance"] = serde_json::json!({"accent": "Teal", "calendar_colors": {"personal": "Pink"}});
    fs::write(t.file(), v.to_string()).unwrap();
    let got = Store::load(&t.file()).unwrap();
    let a = got.settings.appearance;
    assert_eq!((a.mode, a.accent, a.neutral), (Mode::System, Accent::Teal, Neutral::Cool));
    assert_eq!(a.calendar_colors.work, CalendarColor::Blue);
    assert_eq!(a.calendar_colors.personal, CalendarColor::Pink);
    assert_eq!(a.calendar_colors.side, CalendarColor::Amber);
    assert_eq!(got.settings.week_start, Weekday::Sun); // sibling untouched
}

#[test]
fn every_settings_field_falls_back_alone() {
    type Reset = Box<dyn Fn(&mut Store)>;
    let paths: &[(&[&str], Reset)] = &[
        (&["week_start"], Box::new(|s| s.settings.week_start = Weekday::Mon)),
        (&["time_format"], Box::new(|s| s.settings.time_format = TimeFormat::H24)),
        (&["appearance", "mode"], Box::new(|s| s.settings.appearance.mode = Mode::System)),
        (&["appearance", "accent"], Box::new(|s| s.settings.appearance.accent = Accent::Red)),
        (&["appearance", "neutral"], Box::new(|s| s.settings.appearance.neutral = Neutral::Cool)),
        (&["appearance", "calendar_colors", "work"], Box::new(|s| s.settings.appearance.calendar_colors.work = CalendarColor::Blue)),
        (&["appearance", "calendar_colors", "personal"], Box::new(|s| s.settings.appearance.calendar_colors.personal = CalendarColor::Green)),
        (&["appearance", "calendar_colors", "side"], Box::new(|s| s.settings.appearance.calendar_colors.side = CalendarColor::Amber)),
    ];
    for (path, reset) in paths {
        for bad in [json!("nonsense"), json!(12), json!(null), json!([1])] {
            let t = Tmp::new();
            let mut v = serde_json::to_value(custom()).unwrap();
            let mut node = &mut v["settings"];
            for k in &path[..path.len() - 1] {
                node = &mut node[*k];
            }
            node[path[path.len() - 1]] = bad.clone();
            write_json(&t, &v);
            let mut want = custom();
            reset(&mut want);
            assert_eq!(Store::load(&t.file()).unwrap(), want, "{path:?} = {bad}");
        }
    }
}

#[test]
fn calendars_fall_back_per_entry() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["calendars"] = json!([{"id": 0, "visible": false}, {"id": 1, "visible": "yes"}, 5, {"id": 99, "visible": false}, {"id": 256, "visible": false}]);
    write_json(&t, &v);
    let c = Store::load(&t.file()).unwrap().calendars;
    assert_eq!(c.len(), 3);
    assert!(!c[0].visible && c[1].visible && c[2].visible);
    assert_eq!(c[2].name, "Side project");
    assert_eq!(baks(&t.0).len(), 1);
}

#[test]
fn duplicate_calendar_id_first_wins() {
    let t = Tmp::new();
    let mut v = serde_json::to_value(custom()).unwrap();
    v["calendars"] = json!([{"id": 0, "visible": false}, {"id": 0, "visible": true}]);
    write_json(&t, &v);
    let c = Store::load(&t.file()).unwrap().calendars;
    assert!(!c[0].visible);
}

#[test]
fn event_on_unknown_calendar_is_dropped_others_kept() {
    let t = Tmp::new();
    let mut s = custom();
    let n = s.events.len();
    s.events[0].calendar = 42;
    s.save(&t.file()).unwrap();
    let got = Store::load(&t.file()).unwrap().events;
    assert_eq!(got.len(), n - 1);
    assert!(got.iter().all(|e| e.calendar < 3));
}

#[test]
fn settings_repairs_back_up_the_original() {
    let bad_settings: Vec<Mutate> = vec![
        Box::new(|v| v["settings"]["appearance"]["accent"] = "Chartreuse".into()),
        Box::new(|v| v["settings"]["week_start"] = 7.into()),
        Box::new(|v| v["settings"]["time_format"] = Value::Null),
        Box::new(|v| v["settings"]["appearance"]["calendar_colors"]["side"] = true.into()),
        Box::new(|v| v["settings"] = "oops".into()),
        Box::new(|v| v["settings"]["appearance"] = json!([1])),
        Box::new(|v| v["settings"]["appearance"]["calendar_colors"] = json!(3)),
    ];
    for (i, f) in bad_settings.iter().enumerate() {
        let t = Tmp::new();
        let mut v = serde_json::to_value(custom()).unwrap();
        f(&mut v);
        write_json(&t, &v);
        Store::load(&t.file()).unwrap();
        let b = baks(&t.0);
        assert_eq!(b.len(), 1, "case {i}");
        assert_eq!(fs::read_to_string(&b[0]).unwrap(), v.to_string(), "case {i}");
    }
}

#[test]
fn calendar_entry_with_bad_visible_is_kept_with_default() {
    for entry in [json!({"id": 0}), json!({"id": 0, "visible": "no"}), json!({"id": 0, "visible": null})] {
        let t = Tmp::new();
        let mut v = serde_json::to_value(custom()).unwrap();
        v["calendars"] = json!([entry, {"id": 0, "visible": false}, {"id": 1, "visible": false}]);
        write_json(&t, &v);
        let c = Store::load(&t.file()).unwrap().calendars;
        assert!(c[0].visible, "{entry}: bad entry keeps default, and its id is taken so the later duplicate loses");
        assert!(!c[1].visible, "{entry}: other entries still apply");
        assert_eq!(baks(&t.0).len(), 1, "{entry}");
    }
}

#[test]
fn calendar_entry_without_usable_id_is_dropped_and_backed_up() {
    for entry in [json!({"visible": false}), json!({"id": "x", "visible": false}), json!({"id": 99, "visible": false})] {
        let t = Tmp::new();
        let mut v = serde_json::to_value(custom()).unwrap();
        v["calendars"] = json!([entry]);
        write_json(&t, &v);
        assert!(Store::load(&t.file()).unwrap().calendars.iter().all(|c| c.visible), "{entry}");
        assert_eq!(baks(&t.0).len(), 1, "{entry}");
    }
}

#[test]
fn unknown_keys_make_the_load_lossy_and_back_up_the_original() {
    let extra: Vec<Mutate> = vec![
        Box::new(|v| v["from_the_future"] = json!({"a": 1})),
        Box::new(|v| v["events"][0]["location"] = "Room 4".into()),
        Box::new(|v| v["calendars"][0]["color"] = "red".into()),
        Box::new(|v| v["settings"]["density"] = "compact".into()),
        Box::new(|v| v["settings"]["appearance"]["font"] = "mono".into()),
        Box::new(|v| v["settings"]["appearance"]["calendar_colors"]["extra"] = "Red".into()),
    ];
    for (i, f) in extra.iter().enumerate() {
        let t = Tmp::new();
        let mut v = serde_json::to_value(custom()).unwrap();
        f(&mut v);
        write_json(&t, &v);
        assert_eq!(Store::load(&t.file()).unwrap(), custom(), "case {i}: known data still loads");
        let b = baks(&t.0);
        assert_eq!(b.len(), 1, "case {i}");
        assert_eq!(fs::read_to_string(&b[0]).unwrap(), v.to_string(), "case {i}");
    }
}
