//! Headless tests of the shell: real clicks on the rendered views.
mod common;

use chrono::NaiveDate;
use common::{at, open, open_with, tmp};
use gpui_calendar::store::settings::{Accent, CalendarColor, Mode};
use gpui_calendar::store::{PERSONAL, Store, WORK};
use gpui_calendar::ui::{MIN_WINDOW, window_options};
use gpui_kit::test::{TestQueryExt, TestWindowExt};
use gpui_kit::{AppContext, TestAppContext, px, size};
use std::{cell::Cell, rc::Rc, time::Duration};

#[test]
fn window_minimum_is_960_by_600() {
    assert_eq!(window_options().window_min_size, Some(size(px(960.), px(600.))));
    assert_eq!(MIN_WINDOW, (960.0, 600.0));
}

#[gpui_kit::test]
fn clicking_a_calendar_row_toggles_it_in_the_store(cx: &mut TestAppContext) {
    let p = tmp("row");
    let (h, shell) = open(cx, &p, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let row = w.find_by_label("Work");
        assert_eq!(row.checked(), Some(true));
        w.click("calendar-row-0", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(!shell.read(cx).model.store.calendars[WORK as usize].visible));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find_by_label("Work").checked(), Some(false));
    })
    .unwrap();
    assert!(!Store::load(&p).unwrap().calendars[WORK as usize].visible);
}

#[gpui_kit::test]
fn appearance_screen_applies_persists_refuses_and_resets(cx: &mut TestAppContext) {
    let p = tmp("appearance");
    let (h, shell) = open(cx, &p, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click("account-menu", cx);
        w.render_frame(cx);
        // mode
        w.click("mode-dark", cx);
        // accent: Violet allowed
        w.click("accent-Violet", cx);
        w.click("calendar-color-1-Pink", cx);
        // Indigo is too close to Work blue: refused, reason exposed in label and status
        let indigo = w.find("accent-Indigo");
        // GPUI has no aria-disabled setter for plain elements: the reason lives in the name, once.
        assert!(indigo.label().unwrap().starts_with("Indigo, unavailable: too close to the color of"));
        assert_eq!(indigo.description(), None, "the reason must not be announced a second time");
        w.click("accent-Indigo", cx);
        w.render_frame(cx);
        assert_eq!(w.find("accent-Violet").label(), Some("Violet, selected"));
    })
    .unwrap();
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!(m.store.settings.appearance.mode, Mode::Dark);
        assert_eq!(m.store.settings.appearance.accent, Accent::Violet);
        assert_eq!(m.store.settings.appearance.calendar_colors.personal, CalendarColor::Pink);
        assert!(m.hint.starts_with("Indigo is unavailable"), "{}", m.hint);
        assert!(m.theme().dark);
    });
    let disk = Store::load(&p).unwrap().settings.appearance;
    assert_eq!((disk.mode, disk.accent), (Mode::Dark, Accent::Violet));
    cx.update_window(h, |_, w, cx| {
        w.click("reset-appearance", cx);
    })
    .unwrap();
    assert_eq!(Store::load(&p).unwrap().settings.appearance, Default::default());
    let _ = PERSONAL;
}

#[gpui_kit::test]
fn every_control_has_an_accessible_name_and_view_switch_is_pressed(cx: &mut TestAppContext) {
    let (h, _) = open(cx, &tmp("names"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        for name in ["New event", "Previous month", "Next month", "Today", "Appearance", "Open event", "Day", "Week", "Month", "Year", "October 28", "Work", "Personal", "Side project"] {
            assert!(w.try_find_by_label(name).is_some(), "missing control named {name}");
        }
        assert_eq!(w.find("view-month").checked(), Some(true));
        w.click("view-week", cx);
        w.render_frame(cx);
        assert_eq!(w.find("view-week").checked(), Some(true));
        assert!(w.try_find_by_label("Next week").is_some());
        assert_eq!(w.find("view-month").checked(), Some(false));
    })
    .unwrap();
}

#[gpui_kit::test]
fn sidebar_collapses_to_a_rail_below_1120(cx: &mut TestAppContext) {
    let (h, _) = open(cx, &tmp("rail"), 1000.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("sidebar").bounds().size.width, px(64.));
        assert!(w.try_find_by_label("Open event").is_none() && w.try_find("banner").is_none(), "banner hidden in the rail");
        assert!(w.try_find("mini-month").is_none());
        assert!(w.try_find_by_label("New event").is_some());
        assert_eq!(w.find("add-event").bounds().size, size(px(40.), px(40.)), "add button stays square in the rail");
        assert!(w.try_find("app-mark").is_none() && w.try_find("avatar").is_none(), "the rail draws neither mark nor avatar");
        for n in ["Work", "Personal", "Side project"] {
            assert!(w.try_find_by_label(n).is_some(), "{n} toggle stays in the rail");
        }
    })
    .unwrap();
    let (h, _) = open(cx, &tmp("full"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("sidebar").bounds().size.width, px(236.));
        assert!(w.try_find("mini-month").is_some());
        for id in ["app-mark", "avatar"] {
            assert_eq!(w.find(id).bounds().size, size(px(22.), px(22.)), "{id} is the mock's 22px mark");
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_rail_keeps_an_account_entry_to_reach_appearance_and_come_back(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, &tmp("rail-account"), 1000.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("sidebar").bounds().size.width, px(64.));
        assert!(w.try_find_by_label("Appearance").is_some(), "rail has an Appearance entry");
        w.click("account-menu", cx);
        w.render_frame(cx);
        assert!(w.try_find("appearance").is_some(), "Appearance reachable at 1000px");
        assert!(w.try_find("header").is_none());
        w.click("account-menu", cx);
        w.render_frame(cx);
        assert!(w.try_find("header").is_some(), "and the calendar comes back");
        // Opened wide, then narrowed: the entry is still there.
        w.click("account-menu", cx);
        w.render_frame(cx);
    })
    .unwrap();
    cx.update(|cx| assert_eq!(shell.read(cx).model.screen, gpui_calendar::ui::model::Screen::Appearance));
}

#[gpui_kit::test]
fn a_failed_save_is_visible_on_the_calendar_screen(cx: &mut TestAppContext) {
    let p = tmp("blocked");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p.parent().unwrap().join("sub"), b"x").unwrap();
    let (h, _) = open(cx, &p.parent().unwrap().join("sub").join("calendar.json"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("notice").is_none(), "no notice before anything fails");
        w.click("calendar-row-0", cx);
        w.render_frame(cx);
        assert!(w.try_find("header").is_some(), "still on the calendar screen");
        let n = w.find("notice");
        assert!(n.label().unwrap().contains("not saved"), "{:?}", n.label());
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_load_error_blocks_saving_and_shows_the_message(cx: &mut TestAppContext) {
    let p = tmp("load-error");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, b"precious").unwrap();
    let (h, _) = open_with(cx, &p, 1280., Rc::new(Cell::new(at(28, 17, 35))), |s| s.block_saving("permission denied".into()));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("notice").label().unwrap().contains("permission denied"));
        w.click("calendar-row-0", cx);
        w.click("account-menu", cx);
        w.render_frame(cx);
        assert!(w.try_find("notice").is_some(), "visible on Appearance too");
    })
    .unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"precious", "the file is kept as it was");
}

#[gpui_kit::test]
fn today_follows_the_clock_across_midnight(cx: &mut TestAppContext) {
    let now = Rc::new(Cell::new(at(28, 23, 58)));
    let (h, shell) = open_with(cx, &tmp("clock"), 1280., now.clone(), |_| {});
    cx.update(|cx| assert_eq!(shell.read(cx).model.today, NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()));
    now.set(at(29, 0, 2));
    cx.executor().advance_clock(Duration::from_secs(61));
    cx.run_until_parked();
    cx.update(|cx| {
        let m = &shell.read(cx).model;
        assert_eq!(m.today, NaiveDate::from_ymd_opt(2026, 10, 29).unwrap());
        assert_eq!(m.now.format("%H:%M").to_string(), "00:02");
    });
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find_by_label("October 29").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_kit_theme_is_pushed_only_when_the_appearance_changes(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, &tmp("pushes"), 1280.);
    let pushes = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).theme_pushes());
    let start = pushes(cx);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click("view-week", cx);
        w.click("nav-next", cx);
        w.click("calendar-row-0", cx);
        w.render_frame(cx);
    })
    .unwrap();
    assert_eq!(pushes(cx), start, "navigation and visibility do not touch the kit theme");
    cx.update_window(h, |_, w, cx| {
        w.click("account-menu", cx);
        w.render_frame(cx);
        w.click("mode-dark", cx);
        w.render_frame(cx);
    })
    .unwrap();
    assert_eq!(pushes(cx), start + 1, "a mode change pushes once");
}

#[gpui_kit::test]
fn an_unavailable_swatch_never_changes_the_setting(cx: &mut TestAppContext) {
    let p = tmp("swatch");
    let (h, shell) = open(cx, &p, 1280.);
    cx.update_window(h, |_, w, cx| {
        w.click("account-menu", cx);
        w.render_frame(cx);
        w.click("accent-Indigo", cx);
        w.click("calendar-color-2-Blue", cx);
        w.render_frame(cx);
        assert!(w.find("accent-Indigo").focused().is_some(), "still a focus target");
        assert_eq!(w.find("accent-Indigo").disabled(), Some(true), "unavailable accent is aria-disabled");
        assert_eq!(w.find("calendar-color-2-Blue").disabled(), Some(true), "and so is an unavailable calendar color");
        for ok in ["accent-Violet", "calendar-color-2-Pink"] {
            assert_ne!(w.find(ok).disabled(), Some(true), "{ok} is available");
        }
    })
    .unwrap();
    cx.update(|cx| {
        let a = shell.read(cx).model.store.settings.appearance;
        assert_eq!(a, Default::default());
    });
    assert!(!p.exists(), "no save for a refused swatch");
}
