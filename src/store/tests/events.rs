use super::*;

fn event_json(extra: Value) -> Value {
    let mut e = json!({"id": 900, "title": "X", "calendar": 1, "start": "2027-01-05T09:00:00", "end": "2027-01-05T10:00:00", "all_day": false});
    for (k, v) in extra.as_object().unwrap() {
        if v.is_null() {
            e.as_object_mut().unwrap().remove(k);
        } else {
            e[k] = v.clone();
        }
    }
    e
}

fn load_events(events: Vec<Value>) -> (Vec<Event>, usize) {
    let t = Tmp::new();
    write_json(&t, &json!({"version": 1, "events": events}));
    let got = Store::load(&t.file()).unwrap().events;
    (got, baks(&t.0).len())
}

#[test]
fn wrong_typed_all_day_keeps_event_with_default() {
    let (got, nbak) = load_events(vec![event_json(json!({"all_day": "yes"}))]);
    assert_eq!(got.len(), 1);
    assert!(!got[0].all_day);
    assert_eq!((got[0].id, got[0].title.as_str(), got[0].calendar), (900, "X", 1));
    assert_eq!(nbak, 1);
}

#[test]
fn wrong_typed_title_keeps_event_with_empty_title() {
    let (got, _) = load_events(vec![event_json(json!({"title": 5}))]);
    assert_eq!((got.len(), got[0].title.as_str()), (1, ""));
}

#[test]
fn required_fields_missing_or_bad_drop_the_event() {
    for bad in [json!({"id": null}), json!({"id": "x"}), json!({"calendar": null}), json!({"calendar": 99}), json!({"calendar": 300}), json!({"start": null}), json!({"start": "soon"})] {
        let (got, nbak) = load_events(vec![event_json(bad.clone())]);
        assert!(got.is_empty(), "{bad}");
        assert_eq!(nbak, 1, "{bad}");
    }
}

#[test]
fn end_not_after_start_is_repaired() {
    let at = |h, m| chrono::NaiveDate::from_ymd_opt(2027, 1, 5).unwrap().and_hms_opt(h, m, 0).unwrap();
    for end in [json!("2027-01-05T09:00:00"), json!("2027-01-05T08:00:00"), json!("garbage"), Value::Null] {
        let (got, nbak) = load_events(vec![event_json(json!({"end": end}))]);
        assert_eq!(got[0].start, at(9, 0));
        assert_eq!(got[0].end, at(9, 15), "{end}");
        assert_eq!(nbak, 1);
    }
    let (got, _) = load_events(vec![event_json(json!({"all_day": true, "start": "2027-01-05T00:00:00", "end": "2027-01-05T00:00:00"}))]);
    assert_eq!(got[0].end, at(0, 0) + chrono::Duration::days(1));
}

#[test]
fn valid_end_is_kept_untouched() {
    let (got, nbak) = load_events(vec![event_json(json!({}))]);
    assert_eq!(got[0].end, chrono::NaiveDate::from_ymd_opt(2027, 1, 5).unwrap().and_hms_opt(10, 0, 0).unwrap());
    assert_eq!(nbak, 0);
}

#[test]
fn duplicate_ids_first_keeps_id_later_get_fresh_ids() {
    let a = event_json(json!({"id": 5, "title": "A"}));
    let b = event_json(json!({"id": 5, "title": "B"}));
    let c = event_json(json!({"id": 5, "title": "C"}));
    let (got, nbak) = load_events(vec![a, b, c]);
    let ids: Vec<_> = got.iter().map(|e| (e.id, e.title.as_str())).collect();
    assert_eq!(ids, [(5, "A"), (6, "B"), (7, "C")]);
    assert_eq!(nbak, 1);
}

#[test]
fn duplicate_id_repair_skips_ids_in_use() {
    let evs = vec![event_json(json!({"id": 5})), event_json(json!({"id": 5, "title": "B"})), event_json(json!({"id": 6, "title": "C"}))];
    let (got, _) = load_events(evs);
    let mut ids: Vec<_> = got.iter().map(|e| e.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3);
    assert_eq!(got[2].id, 6, "unique id is never renumbered");
}
