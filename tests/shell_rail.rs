//! The rail boundary: 1119 is a rail, 1120 is the full sidebar, and the minimum window still reaches Appearance.
mod common;

use common::{open, tmp};
use gpui_calendar::store::Saved;
use gpui_kit::test::{TestQueryExt, TestWindowExt};
use gpui_kit::{AppContext, TestAppContext, px};

fn sidebar_width(cx: &mut TestAppContext, name: &str, w: f32) -> gpui_kit::Pixels {
    let (h, _) = open(cx, &tmp(name), w);
    cx.update_window(h, |_, win, cx| {
        win.render_frame(cx);
        win.find("sidebar").bounds().size.width
    })
    .unwrap()
}

#[gpui_kit::test]
fn the_sidebar_is_a_rail_at_1119_and_full_at_1120(cx: &mut TestAppContext) {
    assert_eq!(sidebar_width(cx, "w1119", 1119.), px(64.));
    assert_eq!(sidebar_width(cx, "w1120", 1120.), px(236.));
}

#[gpui_kit::test]
fn the_rail_at_the_minimum_width_still_reaches_appearance(cx: &mut TestAppContext) {
    let (h, _) = open(cx, &tmp("w960"), 960.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("sidebar").bounds().size.width, px(64.));
        w.click("account-menu", cx);
        w.render_frame(cx);
        assert!(w.try_find("appearance").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_not_durable_save_shows_its_notice_on_screen(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, &tmp("nodur"), 1280.);
    cx.update(|cx| {
        shell.update(cx, |s, cx| {
            assert!(!s.model.record_save(Ok(Saved::NotDurable(std::io::Error::other("fsync failed")))));
            s.sync(cx);
        })
    });
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(
            w.find("notice").label(),
            Some("Saved, but it may be lost if the computer loses power (fsync failed).")
        );
    })
    .unwrap();
}
