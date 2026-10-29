//! Headless tests of the popover's time fields: a click opens a list of times at 15-minute steps, picked with the pointer or Up, Down, Enter and
//! Escape, never typed as text.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::domain::form::{Draft, Field};
use gpui_calendar::store::settings::TimeFormat;
use gpui_calendar::ui::Shell;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, Role, TestAppContext, px};
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

/// Pick the row `hhmm` of the field's list by clicking it.
fn pick(cx: &mut TestAppContext, h: AnyWindowHandle, f: Field, hhmm: &str) {
    open_list(cx, h, f);
    cx.update_window(h, |_, w, cx| {
        w.click(row(f, hhmm), cx);
        w.render_frame(cx);
    })
    .unwrap();
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

fn gone(cx: &mut TestAppContext, h: AnyWindowHandle, id: &'static str) -> bool {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.try_find(id).is_none()
    })
    .unwrap()
}

#[gpui_kit::test]
fn times_are_pickers_named_start_and_end(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-name", TimeFormat::H24);
    cx.update_window(h, |_, w, _| {
        for (f, group) in [(Field::StartTime, "pop-start-time-field"), (Field::EndTime, "pop-end-time-field")] {
            assert_eq!(w.find(trigger(f)).role(), Some(Role::ComboBox), "{f:?} is a picker, not a typed or segmented field");
            assert_eq!(w.find(group).role(), Some(Role::Group));
        }
        assert_eq!(w.find("pop-start-time-field").label(), Some("Start time"));
        assert_eq!(w.find("pop-end-time-field").label(), Some("End time"));
        assert_eq!(w.find(trigger(Field::StartTime)).label(), Some("Start time, 09:30"), "the trigger carries the value");
        assert_eq!(w.find(trigger(Field::EndTime)).label(), Some("End time, 09:45"));
        assert_eq!(w.find("pop-start-time-box").bounds().size.height, px(48.), "the box is 48px like every field");
    })
    .unwrap();
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("09:30", "09:45"), "the fields start on the draft's times");
}

#[gpui_kit::test]
fn a_click_opens_a_list_in_15_minute_steps_scrolled_to_the_value(cx: &mut TestAppContext) {
    let (h, _) = edit(cx, "time-list", TimeFormat::H24);
    assert!(!list_open(cx, h, Field::StartTime), "closed until clicked");
    open_list(cx, h, Field::StartTime);
    cx.update_window(h, |_, w, _| {
        assert_eq!(w.find(list(Field::StartTime)).role(), Some(Role::ListBox));
        for hhmm in ["0000", "0015", "0930", "2345"] {
            assert!(w.try_find(row(Field::StartTime, hhmm)).is_some(), "row {hhmm}");
        }
        assert!(w.try_find(row(Field::StartTime, "0010")).is_none(), "no row between the steps");
        let picked = w.find(row(Field::StartTime, "0930"));
        assert_eq!(picked.label(), Some("09:30"));
        assert_eq!(picked.selected(), Some(true), "the current value is the selected row");
        assert_ne!(w.find(row(Field::StartTime, "0945")).selected(), Some(true));
        let (list, at) = (w.find(list(Field::StartTime)).bounds(), picked.bounds());
        let field = w.find("pop-start-time-box").bounds();
        assert!(list.top() >= field.bottom() && list.top() <= field.bottom() + px(8.) && list.left() == field.left(), "the list {list:?} hangs just under its field {field:?}");
        assert!(at.top() >= list.top() && at.bottom() <= list.bottom(), "the selected row {at:?} is scrolled into the list {list:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn picking_a_row_updates_the_draft_and_closes_the_list(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-pick", TimeFormat::H24);
    pick(cx, h, Field::EndTime, "1100");
    assert!(!list_open(cx, h, Field::EndTime));
    assert_eq!(draft(cx, &shell).end_time, "11:00");
    cx.update_window(h, |_, w, _| assert_eq!(w.find(trigger(Field::EndTime)).label(), Some("End time, 11:00"))).unwrap();
    assert!(is_open(cx, &shell), "the popover stays");
}

#[gpui_kit::test]
fn the_keyboard_opens_moves_picks_and_escapes(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-key", TimeFormat::H24);
    press(cx, h, &shell, Field::StartTime, "down");
    assert!(list_open(cx, h, Field::StartTime), "Down opens");
    press(cx, h, &shell, Field::StartTime, "down");
    press(cx, h, &shell, Field::StartTime, "down");
    press(cx, h, &shell, Field::StartTime, "up");
    press(cx, h, &shell, Field::StartTime, "enter");
    assert_eq!(draft(cx, &shell).start_time, "09:45", "two down, one up from 09:30");
    assert!(!list_open(cx, h, Field::StartTime) && is_open(cx, &shell), "Enter picks and closes; the popover is not saved");
    press(cx, h, &shell, Field::StartTime, "enter");
    assert!(list_open(cx, h, Field::StartTime), "Enter on a closed field opens it");
    press(cx, h, &shell, Field::StartTime, "down");
    press(cx, h, &shell, Field::StartTime, "escape");
    assert!(!list_open(cx, h, Field::StartTime) && is_open(cx, &shell), "the first Escape closes only the list");
    assert_eq!(draft(cx, &shell).start_time, "09:45", "and picks nothing");
    press(cx, h, &shell, Field::StartTime, "escape");
    assert!(!is_open(cx, &shell), "the next Escape closes the popover");
}

#[gpui_kit::test]
fn the_time_format_setting_picks_the_labels(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-12h", TimeFormat::H12);
    assert_eq!(draft(cx, &shell).start_time, "9:30 AM");
    open_list(cx, h, Field::StartTime);
    cx.update_window(h, |_, w, _| {
        assert_eq!(w.find(row(Field::StartTime, "0930")).label(), Some("9:30 AM"));
        assert_eq!(w.find(row(Field::StartTime, "1430")).label(), Some("2:30 PM"));
        assert_eq!(w.find(row(Field::StartTime, "0000")).label(), Some("12:00 AM"));
    })
    .unwrap();
    cx.update_window(h, |_, w, cx| w.click(row(Field::StartTime, "1030"), cx)).unwrap();
    pick(cx, h, Field::EndTime, "1145");
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("10:30 AM", "11:45 AM"));
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!is_open(cx, &shell), "a 12-hour time saves");
    let e = cx.update(|cx| shell.read(cx).model.store.events.iter().find(|e| e.id == 4).cloned()).unwrap();
    assert_eq!((e.start, e.end), (at(28, 10, 30), at(28, 11, 45)));
}

#[gpui_kit::test]
fn the_end_list_starts_at_the_start_and_shows_each_duration(cx: &mut TestAppContext) {
    let (h, _) = edit(cx, "time-end", TimeFormat::H24);
    open_list(cx, h, Field::EndTime);
    cx.update_window(h, |_, w, _| {
        assert!(w.try_find(row(Field::EndTime, "0915")).is_none(), "nothing before the start (09:30)");
        assert_eq!(w.find(row(Field::EndTime, "0930")).label(), Some("09:30, 0 min"));
        assert_eq!(w.find(row(Field::EndTime, "0945")).label(), Some("09:45, 15 min"));
        assert_eq!(w.find(row(Field::EndTime, "1000")).label(), Some("10:00, 30 min"));
        assert_eq!(w.find(row(Field::EndTime, "1030")).label(), Some("10:30, 1 h"));
        assert_eq!(w.find(row(Field::EndTime, "1115")).label(), Some("11:15, 1 h 45 min"));
    })
    .unwrap();
    pick(cx, h, Field::StartTime, "1000");
    open_list(cx, h, Field::EndTime);
    cx.update_window(h, |_, w, _| assert!(w.try_find(row(Field::EndTime, "0945")).is_none(), "the list follows the start time")).unwrap();
}

#[gpui_kit::test]
fn all_day_disables_the_time_fields_and_keeps_their_times(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-allday", TimeFormat::H24);
    pick(cx, h, Field::StartTime, "1030");
    cx.update_window(h, |_, w, cx| {
        for id in ["pop-start-time-field", "pop-end-time-field"] {
            assert_ne!(w.find(id).disabled(), Some(true), "{id} is live");
        }
        w.click("pop-all-day", cx);
        w.render_frame(cx);
    })
    .unwrap();
    for f in [Field::StartTime, Field::EndTime] {
        cx.update_window(h, |_, w, _| assert_eq!(w.find(trigger(f)).disabled(), Some(true), "{f:?}")).unwrap();
    }
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("10:30", "09:45"), "the draft keeps the times");
    cx.update_window(h, |_, w, cx| {
        w.click(trigger(Field::StartTime), cx);
        w.render_frame(cx);
        assert!(w.try_find(list(Field::StartTime)).is_none(), "a disabled field opens nothing");
        for id in ["pop-start-time-field", "pop-end-time-field"] {
            assert_eq!(w.find(id).disabled(), Some(true), "{id}: the group is read as disabled too");
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn all_day_moves_focus_out_of_a_time_field_it_disables(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-allday-focus", TimeFormat::H24);
    cx.update_window(h, |_, w, cx| {
        let focus = shell.read(cx).popover_time_focus(Field::EndTime, cx).expect("a time field");
        focus.focus(w, cx);
        w.render_frame(cx);
        assert!(focus.contains_focused(w, cx), "the end time has focus");
        shell.update(cx, |s, cx| s.popover_toggle_all_day(w, cx)); // not by click, which would take the focus itself
        w.render_frame(cx);
        assert!(!focus.contains_focused(w, cx), "a disabled field cannot keep the focus");
        assert_eq!(w.find("pop-all-day").focused(), Some(true), "the all-day control has it");
    })
    .unwrap();
}

#[gpui_kit::test]
fn an_end_before_the_start_is_refused_next_to_the_end_field(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-order", TimeFormat::H24);
    pick(cx, h, Field::StartTime, "1030"); // after the 09:45 end
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(is_open(cx, &shell), "refused");
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("pop-end-time-error").label(), Some("The end must be after the start."));
        let (group, field, note) = (w.find("pop-end-time-field").bounds(), w.find("pop-end-time-box").bounds(), w.find("pop-end-time-error").bounds());
        assert!(note.top() >= field.bottom(), "the message {note:?} sits under the field's bottom edge {:?}", field.bottom());
        assert!(note.left() >= group.left() && note.right() <= group.right() && note.bottom() <= group.bottom(), "and inside the end time's own group {group:?}");
    })
    .unwrap();
    let events = cx.update(|cx| shell.read(cx).model.store.events.clone());
    assert_eq!(events.iter().find(|e| e.id == 4).map(|e| e.start), Some(at(28, 9, 30)), "nothing saved");
}

#[gpui_kit::test]
fn the_error_goes_when_that_time_changes(cx: &mut TestAppContext) {
    let (h, _) = edit(cx, "time-clear", TimeFormat::H24);
    pick(cx, h, Field::StartTime, "1030");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!gone(cx, h, "pop-end-time-error"));
    pick(cx, h, Field::StartTime, "1100"); // the error is about the end time: the start changing leaves it
    assert!(!gone(cx, h, "pop-end-time-error"));
    pick(cx, h, Field::EndTime, "1230");
    assert!(gone(cx, h, "pop-end-time-error"), "the end changed");
}

#[gpui_kit::test]
fn a_draft_time_that_cannot_be_read_is_refused_not_dropped(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-bad", TimeFormat::H24);
    let mut d = draft(cx, &shell);
    d.start_time = "nope".into();
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_draft(d, w, cx))).unwrap();
    assert_eq!(draft(cx, &shell).start_time, "nope", "the text is kept");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(is_open(cx, &shell));
    assert_eq!(cx.update(|cx| shell.read(cx).popover_errors()).iter().map(|e| e.field).collect::<Vec<_>>(), [Field::StartTime]);
    pick(cx, h, Field::StartTime, "0900"); // the unreadable time has no row; the list opens at 09:00
    assert_eq!(draft(cx, &shell).start_time, "09:00", "picking a time replaces it");
    assert!(cx.update(|cx| shell.read(cx).popover_errors()).is_empty(), "and the error goes");
}

#[gpui_kit::test]
fn tab_walks_the_time_fields_in_reading_order(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-tab", TimeFormat::H24);
    cx.update_window(h, |_, w, cx| {
        shell.read(cx).popover_date_focus(Field::StartDate, cx).expect("a date field").focus(w, cx);
        w.render_frame(cx);
        w.press("tab", cx);
        w.render_frame(cx);
        let start = shell.read(cx).popover_time_focus(Field::StartTime, cx).unwrap();
        assert!(start.contains_focused(w, cx), "start date, then start time");
        w.press("tab", cx);
        w.render_frame(cx);
        assert!(shell.read(cx).popover_date_focus(Field::EndDate, cx).unwrap().is_focused(w), "a time field is one tab stop");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_row_is_picked_by_a_real_press_and_release_in_both_lists(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-mouse", TimeFormat::H24);
    for (f, hhmm, want) in [(Field::StartTime, "1000", "10:00"), (Field::EndTime, "1115", "11:15")] {
        open_list(cx, h, f);
        let at = cx.update_window(h, |_, w, _| w.find(row(f, hhmm)).bounds().center()).unwrap();
        cx.update_window(h, |_, w, cx| w.pointer_down(at, MouseButton::Left, cx)).unwrap();
        assert!(list_open(cx, h, f), "{f:?}: the press alone must not close the list (focus must stay on the field)");
        cx.update_window(h, |_, w, cx| {
            w.pointer_up(at, MouseButton::Left, cx);
            w.render_frame(cx);
        })
        .unwrap();
        let d = draft(cx, &shell);
        assert_eq!(if f == Field::StartTime { d.start_time } else { d.end_time }, want, "{f:?}: press and release picked the row");
        assert!(!list_open(cx, h, f), "{f:?}: and closed the list");
    }
}

#[gpui_kit::test]
fn the_end_list_is_the_whole_day_when_the_days_cannot_be_compared(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-days", TimeFormat::H24);
    let mut d = draft(cx, &shell);
    (d.start_date, d.end_date) = ("nope".into(), "nope".into()); // equal text, but no day at all
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_draft(d, w, cx))).unwrap();
    open_list(cx, h, Field::EndTime);
    cx.update_window(h, |_, w, _| assert!(w.try_find(row(Field::EndTime, "0000")).is_some(), "unreadable days are not one day: no cut at the start time")).unwrap();
    let mut d = draft(cx, &shell);
    (d.start_date, d.end_date) = ("2026-10-28".into(), " 2026-10-28 ".into()); // the same day, written differently
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_draft(d, w, cx))).unwrap();
    open_list(cx, h, Field::EndTime);
    cx.update_window(h, |_, w, _| assert!(w.try_find(row(Field::EndTime, "0000")).is_none(), "one day: the list starts at the start time")).unwrap();
}
