//! Helpers of the Year view tests.
use super::{at, open_sized, tmp};
use chrono::NaiveDate;
use gpui_calendar::domain::event::Event;
use gpui_calendar::store::{PERSONAL, SIDE, WORK};
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, MouseButton, TestAppContext};
use std::{cell::Cell, rc::Rc};

pub fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

pub fn timed(id: u64, title: &str, cal: u8, day: u32, h: u32) -> Event {
    Event::timed(id, title, cal, at(day, h, 0), at(day, h + 1, 0))
}

pub fn evs() -> Vec<Event> {
    vec![timed(1, "Standup", WORK, 14, 9), timed(2, "Dentist", PERSONAL, 14, 12), timed(3, "Release", SIDE, 14, 20), timed(4, "Design review", WORK, 27, 14)]
}

pub fn open_year(cx: &mut TestAppContext, name: &str, w: f32) -> (AnyWindowHandle, Entity<Shell>) {
    open_year_with(cx, name, (w, 900.), evs())
}

pub fn open_year_with(cx: &mut TestAppContext, name: &str, size: (f32, f32), events: Vec<Event>) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), size, Rc::new(Cell::new(at(28, 17, 35))), move |s| {
        s.model.store.events = events;
        s.model.set_view(View::Year);
    });
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

/// Focus the grid with a press on its padding (no day chosen).
pub fn focus_grid(cx: &mut TestAppContext, h: AnyWindowHandle) {
    cx.update_window(h, |_, w, cx| {
        let pad = w.find("year-scroll").bounds().origin + gpui_kit::point(gpui_kit::px(4.), gpui_kit::px(4.));
        w.pointer_down(pad, MouseButton::Left, cx);
        w.pointer_up(pad, MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
}

pub fn keys(cx: &mut TestAppContext, h: AnyWindowHandle, ks: &[&str]) {
    cx.update_window(h, |_, w, cx| {
        ks.iter().for_each(|k| w.key_down(k, false, cx));
        w.render_frame(cx);
    })
    .unwrap();
}

/// The id of the focus ring drawn on the day or month title `id`.
pub fn ring_of(id: &str) -> String {
    id.replace("year-day-", "year-ring-").replace("year-month-", "year-ring-month-")
}

/// Whether the focus ring is drawn on the day or month title `id`.
pub fn ringed(cx: &mut TestAppContext, h: AnyWindowHandle, id: &str) -> bool {
    let ring = ring_of(id);
    cx.update_window(h, move |_, w, _| w.try_find(ring).is_some()).unwrap()
}
