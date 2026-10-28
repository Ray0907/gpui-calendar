//! Headless tests of the popover's time fields: the start and end times are picked in segments with the arrow keys, never typed as text.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::domain::form::{Draft, Field};
use gpui_calendar::store::settings::TimeFormat;
use gpui_calendar::ui::Shell;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, Role, TestAppContext};
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

/// Focus a time field and press `key` once, as the arrow keys edit the selected segment.
fn press(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>, f: Field, key: &str) {
    cx.update_window(h, |_, w, cx| {
        let focus = shell.read(cx).popover_time_focus(f, cx).expect("a time field");
        focus.focus(w, cx);
        w.render_frame(cx);
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
fn times_are_time_fields_named_start_and_end(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-name", TimeFormat::H24);
    for (f, group) in [(Field::StartTime, "pop-start-time-field"), (Field::EndTime, "pop-end-time-field")] {
        let id = cx.update(|cx| shell.read(cx).popover_time_id(f)).expect("a time field");
        cx.update_window(h, |_, w, _| {
            assert_eq!(w.find(id.clone()).role(), Some(Role::TimeInput), "{f:?} is a segmented time field");
            assert_eq!(w.find(group).role(), Some(Role::Group));
        })
        .unwrap();
    }
    cx.update_window(h, |_, w, _| {
        assert_eq!(w.find("pop-start-time-field").label(), Some("Start time"));
        assert_eq!(w.find("pop-end-time-field").label(), Some("End time"));
    })
    .unwrap();
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("09:30", "09:45"), "the fields start on the draft's times");
}

#[gpui_kit::test]
fn an_arrow_key_on_a_segment_updates_the_draft(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-key", TimeFormat::H24);
    press(cx, h, &shell, Field::StartTime, "up");
    assert_eq!(draft(cx, &shell).start_time, "10:30", "the hour went up");
    press(cx, h, &shell, Field::EndTime, "down");
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("10:30", "08:45"), "each field edits its own time");
}

#[gpui_kit::test]
fn the_time_format_setting_picks_the_hour_cycle(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-12h", TimeFormat::H12);
    assert_eq!(draft(cx, &shell).start_time, "9:30 AM");
    press(cx, h, &shell, Field::StartTime, "up");
    press(cx, h, &shell, Field::EndTime, "up");
    press(cx, h, &shell, Field::EndTime, "up");
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("10:30 AM", "11:45 AM"));
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!is_open(cx, &shell), "a 12-hour time saves");
    let e = cx.update(|cx| shell.read(cx).model.store.events.iter().find(|e| e.id == 4).cloned()).unwrap();
    assert_eq!((e.start, e.end), (at(28, 10, 30), at(28, 11, 45)));
}

#[gpui_kit::test]
fn all_day_disables_the_time_fields_and_keeps_their_times(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-allday", TimeFormat::H24);
    press(cx, h, &shell, Field::StartTime, "up");
    cx.update_window(h, |_, w, cx| {
        for id in ["pop-start-time-field", "pop-end-time-field"] {
            assert_ne!(w.find(id).disabled(), Some(true), "{id} is live");
        }
        w.click("pop-all-day", cx);
        w.render_frame(cx);
    })
    .unwrap();
    for f in [Field::StartTime, Field::EndTime] {
        let id = cx.update(|cx| shell.read(cx).popover_time_id(f)).expect("still there, disabled");
        cx.update_window(h, |_, w, _| assert_eq!(w.find(id).disabled(), Some(true), "{f:?}")).unwrap();
    }
    let d = draft(cx, &shell);
    assert_eq!((d.start_time.as_str(), d.end_time.as_str()), ("10:30", "09:45"), "the draft keeps the times");
    press(cx, h, &shell, Field::StartTime, "up");
    assert_eq!(draft(cx, &shell).start_time, "10:30", "a disabled field takes no key");
    cx.update_window(h, |_, w, _| {
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
    press(cx, h, &shell, Field::StartTime, "up"); // 10:30, after the 09:45 end
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
    let (h, shell) = edit(cx, "time-clear", TimeFormat::H24);
    press(cx, h, &shell, Field::StartTime, "up");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!gone(cx, h, "pop-end-time-error"));
    press(cx, h, &shell, Field::StartTime, "up"); // the error is about the end time: the start changing leaves it
    assert!(!gone(cx, h, "pop-end-time-error"));
    press(cx, h, &shell, Field::EndTime, "up"); // 10:45, still before 11:30
    assert!(gone(cx, h, "pop-end-time-error"), "the end changed");
    press(cx, h, &shell, Field::EndTime, "up");
    press(cx, h, &shell, Field::EndTime, "up"); // 12:45
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    assert!(!is_open(cx, &shell));
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
    press(cx, h, &shell, Field::StartTime, "up");
    let d = draft(cx, &shell);
    assert_ne!(d.start_time, "nope", "picking a time replaces it");
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
    })
    .unwrap();
}

#[gpui_kit::test]
fn tab_leaves_a_time_field_after_its_last_segment_and_enter_saves(cx: &mut TestAppContext) {
    let (h, shell) = edit(cx, "time-leave", TimeFormat::H24);
    cx.update_window(h, |_, w, cx| {
        shell.read(cx).popover_time_focus(Field::StartTime, cx).unwrap().focus(w, cx);
        w.render_frame(cx);
        let (end_date, end_time) = (shell.read(cx).popover_date_focus(Field::EndDate, cx).unwrap(), shell.read(cx).popover_time_focus(Field::EndTime, cx).unwrap());
        w.press("tab", cx); // hour to minute: still in the field
        w.render_frame(cx);
        assert!(shell.read(cx).popover_time_focus(Field::StartTime, cx).unwrap().is_focused(w) && !end_date.is_focused(w));
        w.press("tab", cx); // out of the last segment, on to the end date
        w.render_frame(cx);
        assert!(end_date.is_focused(w), "the focus trap takes over");
        end_time.focus(w, cx);
        w.render_frame(cx);
        w.press("enter", cx);
    })
    .unwrap();
    assert!(!is_open(cx, &shell), "Enter in a time field saves");
}
