//! Shared helpers of the headless shell tests.
#![allow(dead_code)]
use chrono::{NaiveDate, NaiveDateTime};
use gpui_calendar::store::Store;
use gpui_calendar::ui::{Clock, Shell, window_options};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext, px, size};
use std::{cell::Cell, rc::Rc};

pub fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gpui-calendar-shell-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d.join("calendar.json")
}

pub fn at(d: u32, h: u32, mi: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 10, d).unwrap().and_hms_opt(h, mi, 0).unwrap()
}

pub fn open(cx: &mut TestAppContext, path: &std::path::Path, w: f32) -> (AnyWindowHandle, Entity<Shell>) {
    open_sized(cx, path, (w, 720.), Rc::new(Cell::new(at(28, 17, 35))), |_| {})
}

pub fn open_with(cx: &mut TestAppContext, path: &std::path::Path, w: f32, now: Rc<Cell<NaiveDateTime>>, setup: impl FnOnce(&mut Shell) + 'static) -> (AnyWindowHandle, Entity<Shell>) {
    open_sized(cx, path, (w, 720.), now, setup)
}

pub fn open_sized(cx: &mut TestAppContext, path: &std::path::Path, (w, h): (f32, f32), now: Rc<Cell<NaiveDateTime>>, setup: impl FnOnce(&mut Shell) + 'static) -> (AnyWindowHandle, Entity<Shell>) {
    cx.update(gpui_kit::init);
    let path = path.to_path_buf();
    let mut opts = window_options();
    opts.window_bounds = Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds { origin: Default::default(), size: size(px(w), px(h)) }));
    cx.update(|cx| {
        let clock: Clock = Box::new(move || now.get());
        gpui_kit::open_window(opts, cx, |window, cx| {
            cx.new(|cx| {
                let mut shell = Shell::new(Store::default(), Some(path), clock, window, cx);
                setup(&mut shell);
                shell
            })
        })
        .unwrap()
    })
}


/// Like `open`, but the store comes from `path` the way the app loads it (`Shell::start`).
pub fn open_start(cx: &mut TestAppContext, path: &std::path::Path, w: f32) -> (AnyWindowHandle, Entity<Shell>) {
    cx.update(gpui_kit::init);
    let path = path.to_path_buf();
    let now = at(28, 17, 35);
    let mut opts = window_options();
    opts.window_bounds = Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds { origin: Default::default(), size: size(px(w), px(720.)) }));
    cx.update(|cx| gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| Shell::start(Some(path), Box::new(move || now), window, cx))).unwrap())
}
