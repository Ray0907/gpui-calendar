//! Headless tests of the save-failure notice: where it sits, how it goes away, what it offers.
mod common;

use std::{cell::Cell, rc::Rc};

use common::{at, open, open_sized, tmp};
use gpui_calendar::store::Store;
use gpui_kit::{AppContext, TestAppContext};
use gpui_kit::test::TestWindowExt;

/// A path whose parent is a plain file: every save fails until `unblock` removes that file.
fn blocked(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let p = tmp(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let file = p.parent().unwrap().join("sub");
    std::fs::write(&file, b"x").unwrap();
    (file.join("calendar.json"), file)
}

#[gpui_kit::test]
fn the_notice_can_be_dismissed_and_comes_back_on_the_next_failure(cx: &mut TestAppContext) {
    let (path, _) = blocked("dismiss");
    let (h, _) = open(cx, &path, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.click("calendar-row-0", cx);
        w.render_frame(cx);
        assert!(w.find("notice").label().unwrap().contains("not saved"));
        assert_eq!(w.find("notice-dismiss").label(), Some("Dismiss notice"));
        w.click("notice-dismiss", cx);
        w.render_frame(cx);
        assert!(w.try_find("notice").is_none(), "dismissed");
        w.click("calendar-row-1", cx);
        w.render_frame(cx);
        assert!(w.try_find("notice").is_some(), "a new failure is not silent");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_later_successful_save_does_not_clear_an_earlier_failure(cx: &mut TestAppContext) {
    let (path, file) = blocked("sticky");
    let (h, _) = open(cx, &path, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.click("calendar-row-0", cx);
        w.render_frame(cx);
        assert!(w.try_find("notice").is_some());
    })
    .unwrap();
    std::fs::remove_file(&file).unwrap(); // the disk problem goes away
    cx.update_window(h, |_, w, cx| {
        w.click("calendar-row-1", cx);
        w.render_frame(cx);
        assert!(w.try_find("notice").is_some(), "only dismissing clears it");
    })
    .unwrap();
    assert!(Store::load(&path).is_ok(), "the second save did reach the disk");
}

#[gpui_kit::test]
fn the_notice_sits_in_the_flow_and_covers_no_control_at_960x600(cx: &mut TestAppContext) {
    let (path, _) = blocked("layout");
    let (h, _) = open_sized(cx, &path, (960., 600.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    cx.update_window(h, |_, w, cx| {
        w.click("account-menu", cx);
        w.render_frame(cx);
        w.click("mode-dark", cx);
        w.render_frame(cx);
        let n = w.find("notice").bounds();
        let main = w.find("main").bounds();
        assert!(n.top() >= main.top() && n.bottom() <= main.bottom(), "inside the card");
        for id in ["reset-appearance", "mode-light", "accent-Violet", "calendar-color-2-Pink", "neutral-Warm", "notice-dismiss"] {
            let b = w.find(id).bounds();
            assert!(b.bottom() <= main.bottom() && b.top() >= main.top(), "{id} fully inside the card: {b:?} vs {main:?}");
            if id != "notice-dismiss" {
                assert!(!b.intersects(&n), "{id} is not covered by the notice");
            }
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_blocked_store_offers_retry_that_loads_the_fixed_file(cx: &mut TestAppContext) {
    let p = tmp("retry");
    std::fs::create_dir_all(&p).unwrap(); // a directory where the file should be: unreadable
    let (h, shell) = common::open_with(cx, &p, 1280., Rc::new(Cell::new(at(28, 17, 35))), |s| s.block_saving("is a directory".into()));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("notice-retry").label(), Some("Retry loading settings"));
        w.click("calendar-row-0", cx); // blocked: still says so, even after a dismiss
        w.click("notice-dismiss", cx);
        w.click("calendar-row-1", cx);
        w.render_frame(cx);
        assert!(w.find("notice").label().unwrap().contains("is a directory"));
    })
    .unwrap();
    std::fs::remove_dir_all(&p).unwrap();
    Store::default().save(&p).unwrap();
    cx.update_window(h, |_, w, cx| {
        w.click("notice-retry", cx);
        w.render_frame(cx);
        // The two rows toggled while saving was off are kept, saved, and the notice says so.
        assert!(w.find("notice").label().unwrap().contains("kept"));
        assert!(w.try_find("notice-retry").is_none());
        w.click("notice-dismiss", cx);
        w.click("calendar-row-0", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(shell.read(cx).model.notice.is_empty()));
    let disk = Store::load(&p).unwrap();
    assert!(disk.calendars[0].visible, "saving works again: Work toggled back on");
    assert!(!disk.calendars[1].visible, "Personal hidden while blocked was kept");
}

#[gpui_kit::test]
fn a_set_aside_or_repaired_file_is_announced_with_its_backup(cx: &mut TestAppContext) {
    let p = tmp("quarantine");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, br#"{"version": 999}"#).unwrap();
    let (h, _) = common::open_start(cx, &p, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let msg = w.find("notice").label().unwrap().to_string();
        assert!(msg.contains("backup") && msg.contains(".bak"), "{msg}");
    })
    .unwrap();
    assert!(p.with_file_name("calendar.json.bak").exists());

    let p = tmp("repaired");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, br#"{"version": 1, "events": [{"id": "nope"}]}"#).unwrap();
    let (h, _) = common::open_start(cx, &p, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("notice").label().unwrap().contains("backup"));
    })
    .unwrap();

    let (h, _) = common::open_start(cx, &tmp("clean"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("notice").is_none(), "a missing file is a normal first run");
    })
    .unwrap();
}
