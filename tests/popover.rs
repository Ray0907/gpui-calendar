//! Headless tests of the event popover: real clicks and keys on the rendered shell.
mod common;

use common::{at, open_sized, open_week, saved, tmp};
use gpui_calendar::domain::event::Event;
use gpui_calendar::domain::form::Field;
use gpui_calendar::store::{PERSONAL, SIDE, Store, WORK};
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, TestAppContext};
use std::{cell::Cell, rc::Rc};

/// October 26's week is row 520 of the month list; chip ids end in it.
const OCT_26: usize = 520;
const STANDUP: &str = "Standup, 09:30, Work";

fn open_month(cx: &mut TestAppContext, name: &str) -> (AnyWindowHandle, Entity<Shell>) {
    open_month_wide(cx, name, 1280.)
}

fn open_month_wide(cx: &mut TestAppContext, name: &str, width: f32) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (width, 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    cx.update_window(h, |_, w, cx| (0..2).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

fn events(cx: &mut TestAppContext, shell: &Entity<Shell>) -> Vec<Event> {
    cx.update(|cx| shell.read(cx).model.store.events.clone())
}

fn event(cx: &mut TestAppContext, shell: &Entity<Shell>, id: u64) -> Option<Event> {
    events(cx, shell).into_iter().find(|e| e.id == id)
}

fn is_open(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    cx.update(|cx| shell.read(cx).popover_draft(cx).is_some())
}

fn set(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>, field: Field, text: &str) {
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| s.popover_set(field, text, w, cx));
        w.render_frame(cx);
    })
    .unwrap();
}

fn click(cx: &mut TestAppContext, h: AnyWindowHandle, id: &'static str) {
    cx.update_window(h, |_, w, cx| w.click(id, cx)).unwrap();
}

fn on_disk(name: &str) -> Vec<Event> {
    Store::load(&saved(name)).expect("saved file").events
}

#[gpui_kit::test]
fn clicking_a_month_chip_opens_the_popover_on_that_event(cx: &mut TestAppContext) {
    // The 720px card only fits beside a chip in a wide window; narrower ones center it (next test).
    let (h, shell) = open_month_wide(cx, "pop-open", 1920.);
    assert!(!is_open(cx, &shell));
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    assert!(is_open(cx, &shell));
    let d = cx.update(|cx| shell.read(cx).popover_draft(cx)).unwrap();
    assert_eq!((d.id, d.title.as_str(), d.start_time.as_str(), d.end_time.as_str(), d.calendar), (Some(4), "Standup", "09:30", "09:45", WORK));
    assert_eq!(cx.update(|cx| shell.read(cx).popover_by_key()), Some(false), "a pointer open may animate");
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let chip = w.find_by_label(STANDUP).bounds();
        let card = w.find("popover").bounds();
        assert!(card.left() >= chip.right(), "beside the chip, not over it");
        assert_eq!(w.find("popover").label(), Some("Edit event form"));
        assert!(w.try_find("pop-delete").is_some(), "an existing event can be deleted");
    })
    .unwrap();
}

#[gpui_kit::test]
fn edit_title_and_time_then_save_persists(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-save");
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    set(cx, h, &shell, Field::Title, "Daily sync");
    set(cx, h, &shell, Field::StartTime, "10:00");
    set(cx, h, &shell, Field::EndTime, "10:30");
    assert_eq!(event(cx, &shell, 4).unwrap().title, "Standup", "nothing changes before Save");
    click(cx, h, "pop-save");
    assert!(!is_open(cx, &shell));
    let e = event(cx, &shell, 4).unwrap();
    assert_eq!((e.title.as_str(), e.start, e.end), ("Daily sync", at(28, 10, 0), at(28, 10, 30)));
    assert_eq!(on_disk("pop-save").iter().find(|e| e.id == 4), Some(&e), "the file has it too");
}

#[gpui_kit::test]
fn delete_shows_the_toast_and_undo_restores_the_event_in_place(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-delete");
    let before = events(cx, &shell);
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    click(cx, h, "pop-delete");
    assert!(!is_open(cx, &shell));
    assert!(event(cx, &shell, 4).is_none());
    assert!(on_disk("pop-delete").iter().all(|e| e.id != 4));
    assert_eq!(cx.update(|cx| shell.read(cx).toast_text().map(str::to_string)), Some("Event deleted.".into()));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("toast-undo").label(), Some("Undo"));
        w.click("toast-undo", cx);
        w.render_frame(cx);
        assert!(w.try_find("toast").is_none(), "the toast goes");
    })
    .unwrap();
    assert_eq!(events(cx, &shell), before, "same events in the same order");
    assert_eq!(on_disk("pop-delete"), before);
}

#[gpui_kit::test]
fn escape_closes_and_focus_returns_to_the_grid(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-esc");
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        let card = shell.read(cx).popover_focus().unwrap();
        assert!(card.contains_focused(w, cx), "focus moved into the card");
        assert!(!shell.read(cx).grid_focus().is_focused(w));
        w.key_down("escape", false, cx);
        assert!(shell.read(cx).popover_draft(cx).is_none());
        assert!(shell.read(cx).grid_focus().is_focused(w), "back on the grid, whose roving event is the trigger");
        assert_eq!(shell.read(cx).model.event(4).map(|e| e.title.as_str()), Some("Standup"));
    })
    .unwrap();
}

#[gpui_kit::test]
fn an_invalid_form_blocks_save_and_says_why(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-invalid");
    let before = events(cx, &shell);
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    set(cx, h, &shell, Field::Title, "  ");
    click(cx, h, "pop-save");
    assert!(is_open(cx, &shell), "still open");
    assert_eq!(events(cx, &shell), before);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("pop-title-error").label(), Some("Enter a title."));
        assert_eq!(w.find("pop-title-error").role(), Some(gpui_kit::Role::Alert));
        assert_eq!(w.find("pop-title-field").description(), Some("Enter a title."), "the field is described by its error");
    })
    .unwrap();
    set(cx, h, &shell, Field::Title, "Standup");
    set(cx, h, &shell, Field::EndTime, "09:30");
    click(cx, h, "pop-save");
    assert!(is_open(cx, &shell), "an end that is not after the start also blocks");
    cx.update_window(h, |_, w, _| assert_eq!(w.find("pop-end-time-error").label(), Some("The end must be after the start."))).unwrap();
    assert_eq!(events(cx, &shell), before);
}

#[gpui_kit::test]
fn the_all_day_toggle_hides_times_and_saves_an_all_day_event(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-allday");
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        assert!(w.try_find("pop-start-time").is_some());
        assert_eq!(w.find("pop-all-day").checked(), Some(false));
        w.click("pop-all-day", cx);
        w.render_frame(cx);
        assert_eq!(w.find("pop-all-day").checked(), Some(true));
        assert!(w.try_find("pop-start-time").is_none() && w.try_find("pop-end-time").is_none(), "times are gone");
    })
    .unwrap();
    click(cx, h, "pop-save");
    let e = event(cx, &shell, 4).unwrap();
    assert!(e.all_day);
    assert_eq!((e.start, e.end), (at(28, 0, 0), at(29, 0, 0)));
}

#[gpui_kit::test]
fn choosing_another_calendar_changes_the_bar_color_and_the_event(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-cal");
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    // The top bar is the dot color of the calendar chosen in the form.
    let color = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            let s = shell.read(cx);
            s.model.theme().cal(s.popover_draft(cx).unwrap().calendar).map(|k| k.dot).unwrap()
        })
    };
    let first = color(cx);
    cx.update_window(h, |_, w, cx| {
        assert_eq!(w.find("pop-cal-0").checked(), Some(true));
        w.click("pop-cal-2", cx);
        w.render_frame(cx);
        assert_eq!((w.find("pop-cal-0").checked(), w.find("pop-cal-2").checked()), (Some(false), Some(true)));
    })
    .unwrap();
    let second = color(cx);
    assert_ne!(first, second, "the top bar takes the new calendar's dot color");
    assert_eq!(Some(second), cx.update(|cx| shell.read(cx).model.theme().cal(SIDE).map(|k| k.dot)));
    click(cx, h, "pop-save");
    assert_eq!(event(cx, &shell, 4).unwrap().calendar, SIDE);
}

#[gpui_kit::test]
fn enter_on_the_roving_event_opens_it_without_animation(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-key");
    cx.update_window(h, |_, w, cx| {
        shell.read(cx).grid_focus().focus(w, cx);
        w.render_frame(cx);
        w.key_down("right", false, cx);
        w.key_down("enter", false, cx);
    })
    .unwrap();
    assert!(is_open(cx, &shell), "Enter opens the event the arrows landed on");
    assert_eq!(cx.update(|cx| shell.read(cx).popover_by_key()), Some(true), "keyboard opens appear at once");
}

#[gpui_kit::test]
fn the_add_button_opens_a_new_event_and_save_adds_it(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-new");
    let n = events(cx, &shell).len();
    click(cx, h, "add-event");
    let d = cx.update(|cx| shell.read(cx).popover_draft(cx)).unwrap();
    assert_eq!((d.id, d.title.as_str(), d.start_time.as_str(), d.end_time.as_str()), (None, "", "18:00", "19:00"), "the next hour after 17:35");
    cx.update_window(h, |_, w, _| {
        assert_eq!(w.find("popover").label(), Some("New event form"));
        assert!(w.try_find("pop-delete").is_none(), "nothing to delete yet");
    })
    .unwrap();
    click(cx, h, "pop-save");
    assert!(is_open(cx, &shell), "an empty title blocks");
    set(cx, h, &shell, Field::Title, "Call with Sam");
    click(cx, h, "pop-save");
    let all = events(cx, &shell);
    assert_eq!(all.len(), n + 1);
    let e = all.last().unwrap();
    assert_eq!((e.title.as_str(), e.start, e.end, e.calendar), ("Call with Sam", at(28, 18, 0), at(28, 19, 0), WORK));
    assert_eq!(on_disk("pop-new").len(), n + 1);
}

#[gpui_kit::test]
fn the_banner_opens_the_next_event(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-banner");
    click(cx, h, "open-event");
    let d = cx.update(|cx| shell.read(cx).popover_draft(cx)).unwrap();
    assert_eq!((d.title.as_str(), d.calendar), ("Piano lesson", PERSONAL), "the first event after 17:35");
}

#[gpui_kit::test]
fn the_page_behind_is_inert_and_tab_stays_inside(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "pop-modal");
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    click(cx, h, "calendar-row-1");
    cx.update(|cx| assert!(shell.read(cx).model.store.calendars[1].visible, "a click behind the card reaches nothing"));
    assert!(!is_open(cx, &shell), "but it does close the popover");
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        for _ in 0..40 {
            w.key_down("tab", false, cx);
            assert!(shell.read(cx).popover_focus().unwrap().contains_focused(w, cx), "focus never leaves the card");
        }
        for _ in 0..40 {
            w.key_down("shift-tab", false, cx);
            assert!(shell.read(cx).popover_focus().unwrap().contains_focused(w, cx));
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_popover_flips_to_stay_inside_the_window(cx: &mut TestAppContext) {
    let sat = Event::timed(100, "Late call", WORK, at(31, 14, 0), at(31, 15, 0));
    let (h, shell) = open_week(cx, "pop-flip", View::Week, vec![sat]);
    cx.update_window(h, |_, w, cx| w.click("week-ev-100-2026-10-31", cx)).unwrap();
    assert!(is_open(cx, &shell));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let (card, block) = (w.find("popover").bounds(), w.find("week-ev-100-2026-10-31").bounds());
        let win = w.bounds();
        assert!(card.left() >= win.left() && card.right() <= win.right() && card.top() >= win.top() && card.bottom() <= win.bottom(), "{card:?} inside {win:?}");
        assert!(card.right() <= block.left(), "left of the block: card {card:?} block {block:?}");
    })
    .unwrap();
    cx.update_window(h, |_, w, cx| w.pointer_up(w.find("popover").bounds().center(), MouseButton::Left, cx)).unwrap();
}
