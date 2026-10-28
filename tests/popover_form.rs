//! The form as the mock lays it out: sections, the notes box, and notes surviving save, edit and undo.
mod common;

use common::{at, open_sized, saved, tmp};
use gpui_calendar::domain::form::Field;
use gpui_calendar::store::Store;
use gpui_calendar::ui::Shell;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext};
use std::{cell::Cell, rc::Rc};

fn open(cx: &mut TestAppContext, name: &str) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

fn render(cx: &mut TestAppContext, h: AnyWindowHandle) {
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
}

#[gpui_kit::test]
fn the_card_has_the_mocks_three_sections_and_a_labelled_notes_box(cx: &mut TestAppContext) {
    let (h, _shell) = open(cx, "form-sections");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    render(cx, h);
    cx.update_window(h, |_, w, _| {
        let tops: Vec<_> = ["pop-sec-details", "pop-sec-when", "pop-sec-notes"].map(|id| w.find(id).bounds().top()).into();
        assert!(tops[0] < tops[1] && tops[1] < tops[2], "Details, When, Notes in reading order: {tops:?}");
        assert_eq!(w.find_all_by_label("Notes").len(), 1, "announced once");
        assert!(w.try_find("pop-delete").is_none(), "a new event has no Delete link");
    })
    .unwrap();
}

#[gpui_kit::test]
fn notes_typed_into_a_new_event_are_saved_to_disk(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "form-notes-new");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| {
            s.popover_set(Field::Title, "Call", w, cx);
            s.popover_set_notes("Agenda:\nbudget", w, cx);
        });
        w.render_frame(cx);
    })
    .unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).popover_draft(cx)).unwrap().notes, "Agenda:\nbudget");
    cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
    let on_disk = Store::load(&saved("form-notes-new")).unwrap().events;
    assert_eq!(on_disk.iter().find(|e| e.title == "Call").map(|e| e.notes.as_str()), Some("Agenda:\nbudget"));
}

#[gpui_kit::test]
fn editing_shows_the_saved_notes_and_undo_after_a_change_restores_them(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "form-notes-edit");
    let id = cx.update(|cx| shell.read(cx).model.store.events[0].id);
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| {
            s.show_event(id, w, cx);
            s.popover_set_notes("first", w, cx);
        });
        w.click("pop-save", cx);
        shell.update(cx, |s, cx| s.show_event(id, w, cx));
    })
    .unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).popover_draft(cx)).unwrap().notes, "first", "reopening shows the saved notes");
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| s.popover_set_notes("second", w, cx));
        w.click("pop-save", cx);
        shell.update(cx, |s, cx| s.undo_last(cx));
    })
    .unwrap();
    let notes = cx.update(|cx| shell.read(cx).model.store.events.iter().find(|e| e.id == id).map(|e| e.notes.clone()));
    assert_eq!(notes.as_deref(), Some("first"));
}

#[gpui_kit::test]
fn the_subtitle_says_add_for_a_new_event_and_change_for_an_existing_one(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "form-subtitle");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    render(cx, h);
    cx.update_window(h, |_, w, _| assert_eq!(w.find("pop-subtitle").label(), Some("Add it to one of your calendars."))).unwrap();
    let id = cx.update(|cx| shell.read(cx).model.store.events[0].id);
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| s.show_event(id, w, cx));
        w.render_frame(cx);
        assert_eq!(w.find("pop-subtitle").label(), Some("Change it, then save."));
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_narrow_window_never_clips_the_card_or_its_buttons(cx: &mut TestAppContext) {
    for (name, w_px, open_by) in [("form-narrow-n", 480., "n"), ("form-narrow-plus", 480., "add-event"), ("form-narrow-plus-tiny", 360., "add-event")] {
        let (h, _shell) = open_sized(cx, &tmp(name), (w_px, 900.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
        cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
        cx.update_window(h, |_, w, cx| if open_by == "n" { w.key_down("n", false, cx) } else { w.click(open_by, cx) }).unwrap();
        (0..2).for_each(|_| render(cx, h));
        cx.update_window(h, |_, w, _| {
            for id in ["popover", "pop-save", "pop-cancel"] {
                let b = w.find(id).bounds();
                assert!(f32::from(b.left()) >= 0. && f32::from(b.right()) <= w_px, "{name}: {id} {b:?} leaves the {w_px}px window");
            }
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn calendar_counts_follow_the_events_while_the_card_is_open(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "form-counts-live");
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    render(cx, h);
    let said = |cx: &mut TestAppContext| cx.update_window(h, |_, w, _| w.find("pop-cal-0").description().map(str::to_string)).unwrap();
    let n = cx.update(|cx| shell.read(cx).model.store.events.iter().filter(|e| e.calendar == 0).count());
    assert_eq!(said(cx), Some(format!("{n} events")), "a new event counts every event in the store");
    let id = cx.update(|cx| shell.read(cx).model.store.events.iter().find(|e| e.calendar == 0).map(|e| e.id)).unwrap();
    cx.update(|cx| shell.update(cx, |s, cx| { s.model.delete_event(id); cx.notify(); }));
    render(cx, h);
    assert_eq!(said(cx), Some(format!("{} events", n - 1)), "the count is not frozen at open");
}

#[gpui_kit::test]
fn at_360px_the_sections_stack_and_the_date_and_time_fields_stay_usable(cx: &mut TestAppContext) {
    let (h, _shell) = open_sized(cx, &tmp("form-stacked"), (360., 900.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    cx.update_window(h, |_, w, cx| w.click("add-event", cx)).unwrap();
    (0..2).for_each(|_| render(cx, h));
    cx.update_window(h, |_, w, _| {
        let card = w.find("popover").bounds();
        for id in ["pop-start-date-field", "pop-start-time-field"] {
            let b = w.find(id).bounds();
            assert!(f32::from(b.size.width) >= 100., "{id} is only {}px wide", f32::from(b.size.width));
            assert!(b.left() >= card.left() && b.right() <= card.right(), "{id} {b:?} clips out of the card {card:?}");
        }
        let (sec, body) = (w.find("pop-sec-when").bounds(), w.find("pop-start-date-field").bounds());
        assert!(f32::from(body.left() - sec.left()) < 60., "the hint stacks above the fields, not beside them");
    })
    .unwrap();
}

#[gpui_kit::test]
fn a_keyed_card_stays_inside_the_main_area_of_a_600px_window(cx: &mut TestAppContext) {
    let (h, _shell) = open_sized(cx, &tmp("form-main-area"), (600., 900.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    (0..3).for_each(|_| render(cx, h));
    cx.update_window(h, |_, w, _| {
        let (main, card) = (w.find("main").bounds(), w.find("popover").bounds());
        assert!(f32::from(card.left() - main.left()) >= 8. - 0.5 && f32::from(main.right() - card.right()) >= 8. - 0.5, "card {card:?} leaves the main area {main:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn counts_are_keyed_by_calendar_id_not_by_position(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "form-counts-id");
    cx.update(|cx| {
        shell.update(cx, |s, cx| {
            let ev = s.model.store.events[0].clone();
            s.model.store.calendars.push(gpui_calendar::store::Calendar { id: 7, name: "Extra".into(), visible: true });
            s.model.store.events.push(gpui_calendar::domain::event::Event { id: 9001, calendar: 7, ..ev });
            cx.notify();
        })
    });
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    (0..2).for_each(|_| render(cx, h));
    cx.update_window(h, |_, w, _| assert_eq!(w.find("pop-cal-7").description(), Some("1 events"))).unwrap();
}

#[gpui_kit::test]
fn a_tiny_window_does_not_panic_on_negative_room(cx: &mut TestAppContext) {
    let (h, _shell) = open_sized(cx, &tmp("form-tiny"), (200., 40.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    (0..3).for_each(|_| render(cx, h));
    cx.update_window(h, |_, w, _| assert!(f32::from(w.find("popover").bounds().size.height) >= 0.)).unwrap();
}
