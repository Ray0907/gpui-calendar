//! The ghost a create drag shows is the event the drag then stores.
mod common;

use chrono::NaiveDate;
use common::{at, open_sized, tmp};
use gpui_calendar::domain::drag::{Drag, Pos};
use gpui_calendar::domain::event::Event;
use std::{cell::Cell, rc::Rc};

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
}

#[gpui_kit::test]
fn the_create_ghost_equals_the_stored_event_in_month_and_week(cx: &mut gpui_kit::TestAppContext) {
    let (_h, shell) = open_sized(cx, &tmp("ghost"), (1280., 900.), Rc::new(Cell::new(at(28, 13, 20))), |_| {});
    let drags = [
        ("month", Drag::create(Pos::Month(day(27))), Pos::Month(day(29))),
        ("week all-day", Drag::create(Pos::AllDay(day(27))), Pos::AllDay(day(29))),
        ("week timed", Drag::create(Pos::Grid(day(28), 9 * 60 + 7)), Pos::Grid(day(28), 10 * 60 + 20)),
        ("week short", Drag::create(Pos::Grid(day(28), 9 * 60)), Pos::Grid(day(28), 9 * 60 + 1)),
    ];
    for (name, mut drag, to) in drags {
        drag.update(to);
        let p = drag.preview();
        let (ghost, stored) = cx.update(|cx| {
            shell.update(cx, |s, _| {
                let ghost = s.model.ghost_event(&p).expect("a calendar is on");
                let id = s.model.create_from_drag(p).unwrap();
                let stored = s.model.event(id).unwrap().clone();
                (ghost, stored)
            })
        });
        assert_eq!(Event { id: stored.id, ..ghost }, stored, "{name}");
    }
}
