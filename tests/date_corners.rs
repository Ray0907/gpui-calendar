//! The whole 48px date box is the click target, corner pixels included, at rest and with a refused day (2px border).
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::domain::form::{Draft, Field};
use gpui_calendar::store::WORK;
use gpui_calendar::ui::Shell;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext, point, px};
use std::{cell::Cell, rc::Rc};

fn with_draft(cx: &mut TestAppContext, name: &str, start: &str) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    let mut draft = Draft::new_at(at(14, 19, 0), WORK, false);
    draft.title = "Standup".into();
    if !start.is_empty() {
        draft.start_date = start.into();
    }
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_draft(draft, w, cx))).unwrap();
    cx.run_until_parked();
    (h, shell)
}

fn open_at(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>, f: Field, far: bool) -> Option<bool> {
    let id = cx.update(|cx| shell.read(cx).popover_date_id(f)).unwrap();
    let boxed = if f == Field::StartDate { "pop-start-date-box" } else { "pop-end-date-box" };
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let b = w.find(boxed).bounds();
        let (x, y) = if far { (f32::from(b.size.width) - 1., f32::from(b.size.height) - 1.) } else { (1., 1.) };
        w.click_at(boxed, point(px(x), px(y)), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.find(id).expanded()
    })
    .unwrap()
}

#[gpui_kit::test]
fn every_corner_of_the_date_box_opens_the_picker(cx: &mut TestAppContext) {
    // Both states, both corners; a fresh window each time, so a click that misses cannot be read as an earlier one's.
    let mut missed = vec![];
    for (state, start) in [("normal", ""), ("error", "not a day")] {
        for (corner, far) in [("top-left", false), ("bottom-right", true)] {
            let (h, shell) = with_draft(cx, &format!("date-corner-{state}-{corner}"), start);
            if !start.is_empty() {
                cx.update_window(h, |_, w, cx| w.click("pop-save", cx)).unwrap();
                assert!(!cx.update(|cx| shell.read(cx).popover_errors()).is_empty(), "the day is refused");
            }
            if open_at(cx, h, &shell, Field::StartDate, far) != Some(true) {
                missed.push(format!("{state} {corner}"));
            }
        }
    }
    assert!(missed.is_empty(), "a click in these corners did not open the picker: {missed:?}");
}
