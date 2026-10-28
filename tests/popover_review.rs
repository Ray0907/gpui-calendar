//! Review fixes for the event popover: drag release, hidden calendars, close paths, errors, roving calendar cards.
mod common;

use common::{at, open_sized, open_week, tmp};
use gpui_calendar::domain::event::Event;
use gpui_calendar::domain::form::Field;
use gpui_calendar::store::WORK;
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, TestAppContext, point, px};
use std::{cell::Cell, rc::Rc};

const OCT_26: usize = 520;

fn open_month(cx: &mut TestAppContext, name: &str, setup: impl FnOnce(&mut Shell) + 'static) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), setup);
    cx.update_window(h, |_, w, cx| (0..2).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

fn is_open(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    cx.update(|cx| shell.read(cx).popover_draft(cx).is_some())
}

fn event(cx: &mut TestAppContext, shell: &Entity<Shell>, id: u64) -> Event {
    cx.update(|cx| shell.read(cx).model.event(id).cloned().expect("event exists"))
}

#[gpui_kit::test]
fn month_release_in_another_cell_without_a_move_event_still_moves(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-month-up", |_| {});
    let before = event(cx, &shell, 4);
    cx.update_window(h, |_, w, cx| {
        let from = w.find(format!("month-ev-4-{OCT_26}")).bounds().center();
        let to = w.find("month-cell-2026-10-30").bounds().center();
        w.pointer_down(from, MouseButton::Left, cx);
        w.pointer_up(to, MouseButton::Left, cx);
    })
    .unwrap();
    assert!(!is_open(cx, &shell), "a release in another cell is a drop, not a click");
    assert_ne!(event(cx, &shell, 4).start, before.start, "the event moved");
}

#[gpui_kit::test]
fn week_release_on_another_day_without_a_move_event_still_moves(cx: &mut TestAppContext) {
    let review = Event::timed(100, "Review", WORK, at(28, 14, 0), at(28, 15, 0));
    let (h, shell) = open_week(cx, "rev-week-up", View::Week, vec![review]);
    cx.update_window(h, |_, w, cx| {
        let min = |day: u32, m: f32| {
            let b = w.find(format!("week-col-2026-10-{day:02}")).bounds();
            point(b.center().x, b.top() + px(m * 44. / 60.))
        };
        let (from, to) = (min(28, 14. * 60. + 20.5), min(30, 16. * 60. + 20.5));
        w.pointer_down(from, MouseButton::Left, cx);
        w.pointer_up(to, MouseButton::Left, cx);
    })
    .unwrap();
    assert!(!is_open(cx, &shell), "not a click");
    assert_eq!(event(cx, &shell, 100).start.date(), at(30, 0, 0).date());
}

#[gpui_kit::test]
fn add_does_nothing_while_every_calendar_is_hidden(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-hidden", |s| s.model.store.calendars.iter_mut().for_each(|c| c.visible = false));
    cx.update_window(h, |_, w, cx| w.click("add-event", cx)).unwrap();
    assert!(!is_open(cx, &shell), "a new event there would be invisible");
}

#[gpui_kit::test]
fn undo_whose_save_fails_says_so(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-undo-fail", |_| {});
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.click("pop-delete", cx)).unwrap();
    cx.update(|cx| shell.update(cx, |s, _| {
        s.model.dismiss_notice();
        s.model.block_saving("disk full".into());
        s.model.dismiss_notice();
    }));
    cx.update_window(h, |_, w, cx| w.click("toast-undo", cx)).unwrap();
    assert!(cx.update(|cx| !shell.read(cx).model.notice.is_empty()), "the failed save reaches the notice line");
}

#[gpui_kit::test]
fn clicking_outside_the_card_closes_it(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-backdrop", |_| {});
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        let p = point(px(3.), px(3.));
        w.pointer_down(p, MouseButton::Left, cx);
        w.pointer_up(p, MouseButton::Left, cx);
    })
    .unwrap();
    assert!(!is_open(cx, &shell));
}

#[gpui_kit::test]
fn escape_closes_even_when_focus_left_the_card(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-esc", |_| {});
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        shell.read(cx).grid_focus().focus(w, cx);
        w.key_down("escape", false, cx);
    })
    .unwrap();
    assert!(!is_open(cx, &shell));
}

#[gpui_kit::test]
fn a_field_error_goes_when_that_field_is_typed_in(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-err", |_| {});
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        shell.update(cx, |s, cx| {
            s.popover_set(Field::Title, "", w, cx);
            s.popover_set(Field::StartTime, "nope", w, cx);
        });
        w.click("pop-save", cx);
    })
    .unwrap();
    let fields = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).popover_errors().iter().map(|e| e.field).collect::<Vec<_>>());
    assert_eq!(fields(cx), [Field::Title, Field::StartTime]);
    cx.update_window(h, |_, w, cx| {
        w.click("pop-title", cx);
        w.within("pop-title-field").input("S", cx);
    })
    .unwrap();
    assert_eq!(fields(cx), [Field::StartTime], "only the edited field's error goes");
}

#[gpui_kit::test]
fn calendar_cards_are_one_tab_stop_and_arrows_move(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-roving", |_| {});
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let on = |w: &gpui_kit::Window| (0..3).map(|i| w.find(format!("pop-cal-{i}")).focused() == Some(true)).collect::<Vec<_>>();
        w.key_down("tab", false, cx); // title to the cards
        assert_eq!(on(w), [true, false, false], "Tab lands on the chosen card");
        w.key_down("tab", false, cx);
        assert_eq!(on(w), [false, false, false], "and leaves the group: the other cards are no stops");
        w.key_down("shift-tab", false, cx);
        assert_eq!(on(w), [true, false, false], "Shift-Tab comes back to the chosen card");
    })
    .unwrap();
    cx.update_window(h, |_, w, cx| w.click("pop-cal-0", cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.key_down("right", false, cx)).unwrap();
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("pop-cal-1").focused(), Some(true), "focus follows the arrow");
    })
    .unwrap();
    let cal = cx.update(|cx| shell.read(cx).popover_draft(cx).unwrap().calendar);
    assert_eq!(cal, 1, "the arrow chose the next card");
}

fn ids_unique(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    let ids: Vec<u64> = cx.update(|cx| shell.read(cx).model.store.events.iter().map(|e| e.id).collect());
    ids.iter().all(|id| ids.iter().filter(|o| *o == id).count() == 1)
}

fn delete_highest(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>) {
    let top = cx.update(|cx| shell.read(cx).model.store.events.iter().map(|e| e.id).max().unwrap());
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_event(top, w, cx))).unwrap();
    cx.update_window(h, |_, w, cx| w.click("pop-delete", cx)).unwrap();
}

#[gpui_kit::test]
fn undo_after_a_drag_create_never_duplicates_an_id(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-undo-dup", |_| {});
    delete_highest(cx, h, &shell);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let at = |w: &gpui_kit::Window, day| w.find(format!("month-cell-2026-10-{day:02}")).bounds().center();
        let (a, b) = (at(w, 6), at(w, 8));
        w.pointer_down(a, MouseButton::Left, cx);
        w.pointer_move(b, Some(MouseButton::Left), cx);
        w.pointer_up(b, MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
    cx.update(|cx| shell.update(cx, |s, cx| s.undo_last(cx)));
    assert!(ids_unique(cx, &shell), "undo must not put back an id the new event now holds");
}

#[gpui_kit::test]
fn a_drag_edit_clears_the_toast_and_the_undo_history(cx: &mut TestAppContext) {
    let (h, shell) = open_month(cx, "rev-undo-drag", |_| {});
    cx.update_window(h, |_, w, cx| w.click(format!("month-ev-4-{OCT_26}"), cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.click("pop-delete", cx)).unwrap();
    assert!(cx.update(|cx| shell.read(cx).toast_text().is_some()));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let from = w.find(format!("month-ev-5-{OCT_26}")).bounds().center();
        let to = w.find("month-cell-2026-10-30").bounds().center();
        w.pointer_down(from, MouseButton::Left, cx);
        w.pointer_move(to, Some(MouseButton::Left), cx);
        w.pointer_up(to, MouseButton::Left, cx);
    })
    .unwrap();
    assert!(cx.update(|cx| shell.read(cx).toast_text().is_none()), "the offer to undo goes");
    cx.update(|cx| shell.update(cx, |s, cx| s.undo_last(cx)));
    assert!(cx.update(|cx| shell.read(cx).model.event(4).is_none()), "nothing left to undo");
}

#[gpui_kit::test]
fn a_week_drag_edit_clears_the_toast_and_the_undo_history(cx: &mut TestAppContext) {
    let gone = Event::timed(101, "Gone", WORK, at(28, 9, 0), at(28, 10, 0));
    let review = Event::timed(100, "Review", WORK, at(28, 14, 0), at(28, 15, 0));
    let (h, shell) = open_week(cx, "rev-week-undo", View::Week, vec![gone, review]);
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_event(101, w, cx))).unwrap();
    cx.update_window(h, |_, w, cx| w.click("pop-delete", cx)).unwrap();
    assert!(cx.update(|cx| shell.read(cx).toast_text().is_some()));
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let min = |day: u32, m: f32| {
            let b = w.find(format!("week-col-2026-10-{day:02}")).bounds();
            point(b.center().x, b.top() + px(m * 44. / 60.))
        };
        let (from, to) = (min(28, 14. * 60. + 20.5), min(30, 16. * 60. + 20.5));
        w.pointer_down(from, MouseButton::Left, cx);
        w.pointer_up(to, MouseButton::Left, cx);
    })
    .unwrap();
    assert_eq!(event(cx, &shell, 100).start.date(), at(30, 0, 0).date(), "the drag moved it");
    assert!(cx.update(|cx| shell.read(cx).toast_text().is_none()), "the offer to undo goes");
    cx.update(|cx| shell.update(cx, |s, cx| s.undo_last(cx)));
    assert!(cx.update(|cx| shell.read(cx).model.event(101).is_none()), "nothing left to undo");
}
