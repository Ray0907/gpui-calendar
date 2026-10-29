//! The Appearance screen has a way back: a "Back to calendar" button, Escape, the sidebar title and a mini-calendar day. Each returns to the
//! view that was open (day, week, month or year) on the same date.
mod common;

use chrono::NaiveDate;
use common::{at, open_sized, tmp};
use gpui_calendar::ui::model::{Screen, View};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext};
use gpui_calendar::ui::Shell;
use std::{cell::Cell, rc::Rc};

const VIEWS: [View; 4] = [View::Day, View::Week, View::Month, View::Year];

/// A window on `view`, moved off today to Nov 3 so "the same date" means something, and sitting on the Appearance screen.
fn on_appearance(cx: &mut TestAppContext, name: &str, view: View) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1440., 900.), Rc::new(Cell::new(at(28, 17, 35))), move |s| {
        s.model.set_view(view);
        s.model.pick_day(NaiveDate::from_ymd_opt(2026, 11, 3).unwrap());
    });
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click("account-menu", cx);
        w.render_frame(cx);
    })
    .unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.screen, Screen::Appearance));
    (h, shell)
}

fn assert_back(cx: &mut TestAppContext, shell: &Entity<Shell>, view: View, what: &str) {
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!(m.screen, Screen::Calendar, "{what} {view:?}: still on Appearance");
        assert_eq!((m.view, m.anchor), (view, NaiveDate::from_ymd_opt(2026, 11, 3).unwrap()), "{what}: view or date changed");
    });
}

#[gpui_kit::test]
fn the_back_button_returns_to_the_same_view_and_date(cx: &mut TestAppContext) {
    for view in VIEWS {
        let (h, shell) = on_appearance(cx, "back-button", view);
        cx.update_window(h, |_, w, cx| {
            assert_eq!(w.find("back-to-calendar").label(), Some("Back to calendar"));
            w.click("back-to-calendar", cx);
            w.render_frame(cx);
        })
        .unwrap();
        assert_back(cx, &shell, view, "button");
    }
}

#[gpui_kit::test]
fn escape_returns(cx: &mut TestAppContext) {
    for view in VIEWS {
        let (h, shell) = on_appearance(cx, "back-escape", view);
        cx.update_window(h, |_, w, cx| {
            w.press("escape", cx);
            w.render_frame(cx);
        })
        .unwrap();
        assert_back(cx, &shell, view, "escape");
    }
}

#[gpui_kit::test]
fn the_title_returns(cx: &mut TestAppContext) {
    for view in VIEWS {
        let (h, shell) = on_appearance(cx, "back-title", view);
        cx.update_window(h, |_, w, cx| {
            w.click("sidebar-title", cx);
            w.render_frame(cx);
        })
        .unwrap();
        assert_back(cx, &shell, view, "title");
    }
}

#[gpui_kit::test]
fn a_mini_calendar_day_returns_on_that_day(cx: &mut TestAppContext) {
    for view in VIEWS {
        let (h, shell) = on_appearance(cx, "back-mini", view);
        cx.update_window(h, |_, w, cx| {
            w.click("mini-day-2026-11-03", cx);
            w.render_frame(cx);
        })
        .unwrap();
        assert_back(cx, &shell, view, "mini day");
    }
}

#[gpui_kit::test]
fn tab_reaches_the_back_button(cx: &mut TestAppContext) {
    let (h, _) = on_appearance(cx, "back-tab", View::Month);
    cx.update_window(h, |_, w, cx| {
        let mut reached = false;
        for _ in 0..80 {
            w.press("tab", cx);
            w.render_frame(cx);
            if w.try_find("back-to-calendar").is_some_and(|e| e.focused() == Some(true)) {
                reached = true;
                break;
            }
        }
        assert!(reached, "Tab never lands on Back to calendar");
    })
    .unwrap();
}
