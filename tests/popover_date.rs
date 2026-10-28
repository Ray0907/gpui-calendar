//! Headless tests of the popover's date pickers: the start and end days are picked, never typed.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::domain::form::Field;
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, ElementId, Entity, TestAppContext, Window};
use std::{cell::Cell, rc::Rc};

/// October 26's week is row 520 of the month list; chip ids end in it.
const OCT_26: usize = 520;

fn open_month(cx: &mut TestAppContext, name: &str) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    cx.update_window(h, |_, w, cx| (0..2).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

/// Open Standup (Oct 28, 09:30) for editing.
fn edit(cx: &mut TestAppContext, name: &str) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_month(cx, name);
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    (h, shell)
}

fn picker(cx: &mut TestAppContext, shell: &Entity<Shell>, f: Field) -> ElementId {
    cx.update(|cx| shell.read(cx).popover_date_id(f)).expect("a date picker")
}

fn open_picker(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>, f: Field) -> ElementId {
    let id = picker(cx, shell, f);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click(id.clone(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    id
}

fn expanded(cx: &mut TestAppContext, h: AnyWindowHandle, id: &ElementId) -> Option<bool> {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.find(id.clone()).expanded()
    })
    .unwrap()
}

/// Click the day cell labelled `day` (`2026-10-27`) in the open calendar.
fn pick(cx: &mut TestAppContext, h: AnyWindowHandle, day: &str) {
    cx.update_window(h, |_, w: &mut Window, cx| {
        let cell = (0..6).flat_map(|r| (0..7).map(move |c| (r, c))).map(|(r, c)| format!("calendar-{day}-{r}-{c}")).find(|id| w.try_find(id.clone()).is_some());
        w.click(cell.expect("the day is in the calendar"), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn draft(cx: &mut TestAppContext, shell: &Entity<Shell>) -> gpui_calendar::domain::form::Draft {
    cx.update(|cx| shell.read(cx).popover_draft(cx)).expect("popover open")
}

fn is_open(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    cx.update(|cx| shell.read(cx).popover_draft(cx).is_some())
}

fn name(cx: &mut TestAppContext, h: AnyWindowHandle, id: &'static str) -> Option<String> {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.find(id).label().map(str::to_string)
    })
    .unwrap()
}

#[gpui_kit::test]
fn dates_are_buttons_named_with_their_day(cx: &mut TestAppContext) {
    let (h, _shell) = edit(cx, "date-name");
    assert_eq!(name(cx, h, "pop-start-date-field").as_deref(), Some("Start date, 2026-10-28"));
    assert_eq!(name(cx, h, "pop-end-date-field").as_deref(), Some("End date, 2026-10-28"));
}

#[gpui_kit::test]
fn picking_a_day_sets_the_date_and_closes_the_picker(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-pick");
    let id = open_picker(cx, h, &shell, Field::EndDate);
    assert_eq!(expanded(cx, h, &id), Some(true), "a click opens it");
    pick(cx, h, "2026-10-29");
    assert_eq!(expanded(cx, h, &id), Some(false), "picking closes it");
    assert!(is_open(cx, &shell), "the popover stays");
    let d = draft(cx, &shell);
    assert_eq!((d.start_date.as_str(), d.end_date.as_str()), ("2026-10-28", "2026-10-29"));
    assert_eq!(name(cx, h, "pop-end-date-field").as_deref(), Some("End date, 2026-10-29"));
    assert_eq!(d.end_time, "09:45", "times are untouched");
}

#[gpui_kit::test]
fn enter_opens_the_picker_from_the_keyboard(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-enter");
    let id = picker(cx, &shell, Field::StartDate);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        shell.read(cx).popover_date_focus(Field::StartDate, cx).unwrap().focus(w, cx);
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(expanded(cx, h, &id), Some(true));
}

#[gpui_kit::test]
fn escape_closes_only_the_picker_and_focus_returns_to_the_field(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-esc");
    let id = open_picker(cx, h, &shell, Field::StartDate);
    cx.update_window(h, |_, w, cx| w.press("escape", cx)).unwrap();
    cx.run_until_parked();
    assert_eq!(expanded(cx, h, &id), Some(false));
    assert!(is_open(cx, &shell), "the first Esc leaves the popover open");
    cx.update_window(h, |_, w, cx| assert!(shell.read(cx).popover_date_focus(Field::StartDate, cx).unwrap().is_focused(w), "focus is on the field")).unwrap();
    cx.update_window(h, |_, w, cx| w.press("escape", cx)).unwrap();
    assert!(!is_open(cx, &shell), "the second Esc closes the popover");
}

#[gpui_kit::test]
fn shortcuts_stay_silent_while_the_picker_is_open(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-keys");
    let id = open_picker(cx, h, &shell, Field::StartDate);
    let before = cx.update(|cx| shell.read(cx).model.view);
    cx.update_window(h, |_, w, cx| ["w", "y", "t", "n", "left"].iter().for_each(|k| w.press(k, cx))).unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).model.view), before);
    assert_ne!(before, View::Year);
    assert_eq!(draft(cx, &shell).start_date, "2026-10-28");
    assert_eq!(expanded(cx, h, &id), Some(true), "typing did not close it");
}

#[gpui_kit::test]
fn all_day_end_is_inclusive_and_saves_from_the_pickers(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "date-allday");
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| s.show_event(4, w, cx));
        w.click("pop-all-day", cx);
    })
    .unwrap();
    open_picker(cx, h, &shell, Field::EndDate);
    pick(cx, h, "2026-10-30");
    assert_eq!(draft(cx, &shell).end_date, "2026-10-30");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!is_open(cx, &shell));
    let e = cx.update(|cx| shell.read(cx).model.event(4).cloned()).unwrap();
    assert!(e.all_day);
    assert_eq!(e.days().len(), 3, "Oct 28 to Oct 30 inclusive");
}

#[gpui_kit::test]
fn an_end_before_the_start_is_refused_until_another_day_is_picked(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-order");
    open_picker(cx, h, &shell, Field::EndDate);
    pick(cx, h, "2026-10-27");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(is_open(cx, &shell), "refused");
    let errs = cx.update(|cx| shell.read(cx).popover_errors());
    assert!(errs.iter().any(|e| e.field == Field::EndTime), "{errs:?}");
    open_picker(cx, h, &shell, Field::EndDate);
    pick(cx, h, "2026-10-28");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!is_open(cx, &shell), "saved");
}

#[gpui_kit::test]
fn one_escape_from_the_title_closes_the_popover(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-esc-title");
    cx.update_window(h, |_, w, cx| w.press("escape", cx)).unwrap();
    assert!(!is_open(cx, &shell), "no picker is open, so the one Esc is the popover's");
}

#[gpui_kit::test]
fn one_escape_closes_a_keyboard_opened_picker_and_not_the_popover(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "date-esc-key");
    let id = picker(cx, &shell, Field::StartDate);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        shell.read(cx).popover_date_focus(Field::StartDate, cx).unwrap().focus(w, cx);
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(expanded(cx, h, &id), Some(true));
    cx.update_window(h, |_, w, cx| w.press("escape", cx)).unwrap();
    cx.run_until_parked();
    assert_eq!(expanded(cx, h, &id), Some(false));
    assert!(is_open(cx, &shell), "one Esc closes one thing");
}

#[gpui_kit::test]
fn a_date_that_cannot_be_read_stays_visible_and_is_refused_on_save(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "date-bad");
    cx.update_window(h, |_, w, cx| {
        let mut d = gpui_calendar::domain::form::Draft::new_at(at(28, 9, 0), 0, true);
        d.title = "Standup".into();
        d.start_date = "next tuesday".into();
        shell.update(cx, |s, cx| s.show_draft(d, w, cx));
    })
    .unwrap();
    assert_eq!(draft(cx, &shell).start_date, "next tuesday", "the draft keeps what it was given");
    assert_eq!(name(cx, h, "pop-start-date-field").as_deref(), Some("Start date, next tuesday"));
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(is_open(cx, &shell), "refused");
    let errs = cx.update(|cx| shell.read(cx).popover_errors());
    assert!(errs.iter().any(|e| e.field == Field::StartDate), "{errs:?}");
    open_picker(cx, h, &shell, Field::StartDate);
    pick(cx, h, "2026-10-27");
    assert_eq!(draft(cx, &shell).start_date, "2026-10-27", "picking fixes it");
    assert!(cx.update(|cx| shell.read(cx).popover_errors()).iter().all(|e| e.field != Field::StartDate));
}

#[gpui_kit::test]
fn an_error_goes_when_the_day_changes_not_when_it_is_picked_again(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "date-reclear");
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| s.show_event(4, w, cx));
        w.click("pop-all-day", cx);
    })
    .unwrap();
    open_picker(cx, h, &shell, Field::EndDate);
    pick(cx, h, "2026-10-27");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    let on_end = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).popover_errors()).iter().any(|e| e.field == Field::EndDate);
    assert!(on_end(cx), "end before start");
    open_picker(cx, h, &shell, Field::EndDate);
    pick(cx, h, "2026-10-27");
    assert!(on_end(cx), "the same day again changes nothing");
    open_picker(cx, h, &shell, Field::EndDate);
    pick(cx, h, "2026-10-29");
    assert!(!on_end(cx), "another day clears it");
}

#[gpui_kit::test]
fn picking_a_day_clears_only_that_fields_error(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "date-only-own");
    cx.update_window(h, |_, w, cx| {
        let mut d = gpui_calendar::domain::form::Draft::new_at(at(28, 9, 0), 0, true);
        d.title = String::new();
        d.start_date = "next tuesday".into();
        shell.update(cx, |s, cx| s.show_draft(d, w, cx));
        w.click("pop-save", cx);
    })
    .unwrap();
    let has = |cx: &mut TestAppContext, f: Field| cx.update(|cx| shell.read(cx).popover_errors()).iter().any(|e| e.field == f);
    assert!(has(cx, Field::Title) && has(cx, Field::StartDate), "both refused");
    open_picker(cx, h, &shell, Field::StartDate);
    pick(cx, h, "2026-10-27");
    assert!(!has(cx, Field::StartDate), "the picked field's error goes");
    assert!(has(cx, Field::Title), "another field's error stays");
}
