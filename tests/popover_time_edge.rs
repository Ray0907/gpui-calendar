//! Edge cases of the popover's time lists: modified keys stay with the shortcuts, a press on the blank card closes a list, an unreadable time
//! selects no row.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::domain::form::{Draft, Field};
use gpui_calendar::store::settings::TimeFormat;
use gpui_calendar::ui::Shell;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, TestAppContext};
use std::{cell::Cell, rc::Rc};

/// October 26's week is row 520 of the month list; chip ids end in it.
const OCT_26: usize = 520;

/// Standup (Oct 28, 09:30 to 09:45) open for editing.
fn edit(cx: &mut TestAppContext, name: &str, format: TimeFormat) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), move |s| s.model.store.settings.time_format = format);
    cx.update_window(h, |_, w, cx| (0..2).for_each(|_| w.render_frame(cx))).unwrap();
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
    (h, shell)
}

fn draft(cx: &mut TestAppContext, shell: &Entity<Shell>) -> Draft {
    cx.update(|cx| shell.read(cx).popover_draft(cx)).expect("popover open")
}

fn is_open(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    cx.update(|cx| shell.read(cx).popover_draft(cx).is_some())
}

/// The trigger of a time field, the list it opens and one row of it.
fn trigger(f: Field) -> &'static str {
    if f == Field::StartTime { "pop-start-time-trigger" } else { "pop-end-time-trigger" }
}

fn list(f: Field) -> &'static str {
    if f == Field::StartTime { "pop-start-time-list" } else { "pop-end-time-list" }
}

fn row(f: Field, hhmm: &str) -> String {
    format!("{}-opt-{hhmm}", list(f).trim_end_matches("-list"))
}

/// Click the field's box, then let the list paint.
fn open_list(cx: &mut TestAppContext, h: AnyWindowHandle, f: Field) {
    cx.update_window(h, |_, w, cx| {
        w.click(trigger(f), cx);
        w.render_frame(cx);
        w.render_frame(cx);
    })
    .unwrap();
}

fn list_open(cx: &mut TestAppContext, h: AnyWindowHandle, f: Field) -> bool {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.try_find(list(f)).is_some()
    })
    .unwrap()
}

/// Focus the field's trigger and press `key` once.
fn press(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>, f: Field, key: &str) {
    cx.update_window(h, |_, w, cx| {
        let focus = shell.read(cx).popover_time_focus(f, cx).expect("a time field");
        if !focus.is_focused(w) {
            focus.focus(w, cx);
            w.render_frame(cx);
        }
        w.press(key, cx);
        w.render_frame(cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_modified_arrow_or_enter_is_left_to_the_shortcuts(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-mod", TimeFormat::H24);
    for key in ["cmd-down", "ctrl-up", "alt-down", "cmd-enter", "ctrl-space"] {
        press(cx, h, &shell, Field::StartTime, key);
        assert!(!list_open(cx, h, Field::StartTime), "{key} must reach the outer shortcuts, not open the list");
    }
    press(cx, h, &shell, Field::StartTime, "down");
    assert!(list_open(cx, h, Field::StartTime), "the plain key still opens it");
    press(cx, h, &shell, Field::StartTime, "cmd-down");
    press(cx, h, &shell, Field::StartTime, "cmd-enter");
    assert_eq!(draft(cx, &shell).start_time, "09:30", "a modified key neither moves nor picks");
}

#[gpui_kit::test]
fn a_press_on_the_blank_card_closes_the_list(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-outside", TimeFormat::H24);
    for f in [Field::StartTime, Field::EndTime] {
        open_list(cx, h, f);
        assert!(list_open(cx, h, f));
        let at = cx.update_window(h, |_, w, _| w.find("pop-subtitle").bounds().center()).unwrap();
        cx.update_window(h, |_, w, cx| w.pointer_down(at, MouseButton::Left, cx)).unwrap();
        cx.update_window(h, |_, w, cx| w.pointer_up(at, MouseButton::Left, cx)).unwrap();
        assert!(!list_open(cx, h, f), "{f:?}: a press on blank card area leaves no abandoned list");
        assert!(is_open(cx, &shell), "the popover stays");
    }
    // The field itself still toggles, and a press on the list does not close it.
    open_list(cx, h, Field::StartTime);
    cx.update_window(h, |_, w, cx| w.click(trigger(Field::StartTime), cx)).unwrap();
    assert!(!list_open(cx, h, Field::StartTime), "a click on the open field closes its list");
}

#[gpui_kit::test]
fn an_unreadable_time_selects_no_row_and_the_list_does_not_open_on_midnight(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-unread-list", TimeFormat::H24);
    let mut d = draft(cx, &shell);
    d.start_time = "nope".into();
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_draft(d, w, cx))).unwrap();
    open_list(cx, h, Field::StartTime);
    cx.update_window(h, |_, w, _| {
        assert_ne!(w.find(row(Field::StartTime, "0000")).selected(), Some(true), "no time is picked, so no row is selected");
        let list = w.find(list(Field::StartTime)).bounds();
        let nine = w.find(row(Field::StartTime, "0900")).bounds();
        assert!(nine.top() >= list.top() && nine.bottom() <= list.bottom(), "the list opens on 09:00 {nine:?}, in view in {list:?}");
        assert!(w.find(row(Field::StartTime, "0000")).bounds().bottom() <= list.top(), "midnight is scrolled out of view");
    })
    .unwrap();
}
