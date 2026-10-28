//! Headless tests of names, ARIA states and layout at the two smallest widths.
mod common;

use std::{cell::Cell, rc::Rc};

use common::{at, open, open_sized, tmp};
use gpui_kit::test::{TestQueryExt, TestWindowExt};
use gpui_kit::{AppContext, Role, TestAppContext, px};

#[gpui_kit::test]
fn the_appearance_entry_is_named_appearance_in_the_sidebar_and_the_rail(cx: &mut TestAppContext) {
    for w in [1280., 1000.] {
        let (h, _) = open(cx, &tmp(&format!("name{w}")), w);
        cx.update_window(h, |_, win, cx| {
            win.render_frame(cx);
            let a = win.find("account-menu");
            assert_eq!(a.label(), Some("Appearance"), "width {w}");
            assert_eq!(a.expanded(), None, "it opens a screen, not a menu");
            assert!(win.try_find_by_label("Account menu").is_none());
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn the_status_line_has_a_name_and_can_be_observed(cx: &mut TestAppContext) {
    let (h, _) = open(cx, &tmp("status"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.click("account-menu", cx);
        w.render_frame(cx);
        assert_eq!(w.find("status").role(), Some(Role::Status));
        w.click("accent-Indigo", cx);
        w.render_frame(cx);
        assert!(w.find("status").label().unwrap().starts_with("Indigo is unavailable"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn view_and_mini_day_buttons_carry_one_state_attribute_pressed(cx: &mut TestAppContext) {
    let (h, _) = open(cx, &tmp("states"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        for (id, on) in [("view-month", true), ("view-week", false), ("mini-day-2026-10-28", true), ("mini-day-2026-10-27", false)] {
            let b = w.find(id);
            assert_eq!(b.selected(), None, "{id}: selected would conflict with pressed");
            assert_eq!(b.checked(), Some(on), "{id}: pressed state");
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_sidebar_content_fits_at_1120x600_and_960x600(cx: &mut TestAppContext) {
    for w in [1120., 960.] {
        let (h, _) = open_sized(cx, &tmp(&format!("fit{w}")), (w, 600.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
        cx.update_window(h, |_, win, cx| {
            win.render_frame(cx);
            let s = win.find("sidebar").bounds();
            assert!(s.bottom() <= px(600.), "{w}: sidebar inside the window");
            let mut ids = vec!["add-event", "account-menu", "calendar-row-0"];
            if w >= 1120. {
                ids.extend(["mini-month", "mini-day-2026-10-28", "banner"]);
            }
            for id in ids {
                let b = win.find(id).bounds();
                assert!(b.top() >= s.top() && b.bottom() <= s.bottom(), "{w}: {id} {b:?} inside sidebar {s:?}");
            }
            if w >= 1120. {
                let list = win.find("calendar-list").bounds();
                assert!(list.size.height >= px(36.), "{w}: the calendar list keeps at least one row visible: {list:?}");
            }
        })
        .unwrap();
    }
}
