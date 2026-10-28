//! The New event popover as first opened: a clean summary, no errors, and placed inside the main card.
mod common;

use common::{at, open_sized, tmp};
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext};
use gpui_calendar::ui::Shell;
use std::{cell::Cell, rc::Rc};

fn open(cx: &mut TestAppContext, name: &str) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

fn well(cx: &mut TestAppContext, h: AnyWindowHandle) -> String {
    cx.update_window(h, |_, w, _| w.find("pop-well").label().unwrap().to_string()).unwrap()
}

#[gpui_kit::test]
fn a_fresh_new_event_shows_the_time_summary_and_no_error(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "new-clean");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
    assert_eq!(well(cx, h), "Today, 14:00 – 15:00. 1 hour · Wednesday, Oct 28");
    assert!(cx.update(|cx| shell.read(cx).popover_errors().is_empty()));
    cx.update_window(h, |_, w, _| assert!(w.try_find("pop-title-error").is_none(), "no title error before Save")).unwrap();
}

#[gpui_kit::test]
fn saving_without_a_title_shows_the_title_error_and_keeps_the_summary(cx: &mut TestAppContext) {
    let (h, _shell) = open(cx, "new-save");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
    cx.update_window(h, |_, w, _| assert_eq!(w.find("pop-title-error").label(), Some("Enter a title."))).unwrap();
    assert_eq!(well(cx, h), "Today, 14:00 – 15:00. 1 hour · Wednesday, Oct 28");
}

#[gpui_kit::test]
fn an_invalid_time_still_asks_to_check_the_dates(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "new-bad");
    cx.update_window(h, |_, w, cx| {
        w.key_down("n", false, cx);
        shell.update(cx, |s, cx| s.popover_set(gpui_calendar::domain::form::Field::EndTime, "13:00", w, cx));
        w.render_frame(cx);
    })
    .unwrap();
    assert_eq!(well(cx, h), "Check the dates and times");
}

#[gpui_kit::test]
fn n_places_the_card_inside_the_main_card_below_the_header_title(cx: &mut TestAppContext) {
    let (h, _shell) = open(cx, "new-place");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
    cx.update_window(h, |_, w, _| {
        let (card, main, title) = (w.find("popover").bounds(), w.find("main").bounds(), w.find("header-title").bounds());
        assert!(main.contains(&card.origin) && main.contains(&card.bottom_right()), "{card:?} not inside {main:?}");
        assert!(!card.intersects(&title), "{card:?} covers the title {title:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_plus_button_anchors_the_card_to_itself_and_n_uses_the_centred_fallback(cx: &mut TestAppContext) {
    let card = |cx: &mut TestAppContext, name: &str, act: &dyn Fn(&mut gpui_kit::Window, &mut gpui_kit::App)| {
        let (h, _shell) = open(cx, name);
        cx.update_window(h, |_, w, cx| act(w, cx)).unwrap();
        cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
        cx.update_window(h, |_, w, _| (w.find("popover").bounds(), w.find("add-event").bounds())).unwrap()
    };
    let (by_key, button) = card(cx, "new-anchor-n", &|w, cx| w.key_down("n", false, cx));
    let (by_click, _) = card(cx, "new-anchor-plus", &|w, cx| w.click("add-event", cx));
    assert_ne!(by_click, by_key, "the + card sits where the n card does, so it is not anchored to the button");
    let gap = |c: gpui_kit::Bounds<gpui_kit::Pixels>| (c.center() - button.center()).magnitude();
    assert!(gap(by_click) < gap(by_key), "+ card {by_click:?} is no nearer the button {button:?} than the n card {by_key:?}");
}
