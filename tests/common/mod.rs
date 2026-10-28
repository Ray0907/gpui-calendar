//! Shared helpers of the headless shell tests.
#![allow(dead_code)]
pub mod year;
use chrono::{NaiveDate, NaiveDateTime};
use gpui_calendar::store::Store;
use gpui_calendar::ui::{Clock, Shell, window_options};
use gpui_kit::test::TestWindowExt;
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

/// Open the shell on the Week or Day view with exactly `events`, rendered until the grid has scrolled.
pub fn open_week(cx: &mut TestAppContext, name: &str, view: gpui_calendar::ui::model::View, events: Vec<gpui_calendar::domain::event::Event>) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1440., 900.), Rc::new(Cell::new(at(28, 17, 35))), move |s| {
        s.model.store.events = events;
        s.model.set_view(view);
    });
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

/// The store file `tmp(name)` made, without clearing it.
pub fn saved(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("gpui-calendar-shell-{}-{name}", std::process::id())).join("calendar.json")
}

/// A stand-in for a popover: an occluding box over the right `cover_w` px of the window, above the shell.
struct Covered {
    shell: Entity<Shell>,
    cover_w: f32,
}

impl gpui_kit::Render for Covered {
    fn render(&mut self, _: &mut gpui_kit::Window, _: &mut gpui_kit::Context<Self>) -> impl gpui_kit::IntoElement {
        use gpui_kit::{InteractiveElement, ParentElement, Styled, div};
        div().size_full().relative().child(self.shell.clone()).child(div().id("cover").absolute().top_0().right_0().bottom_0().w(px(self.cover_w)).occlude())
    }
}

/// `open_week` with an overlay above the right `cover_w` px of a 1440x900 window.
pub fn open_week_covered(cx: &mut TestAppContext, name: &str, cover_w: f32) -> (AnyWindowHandle, Entity<Shell>) {
    cx.update(gpui_kit::init);
    let path = tmp(name);
    let mut opts = window_options();
    opts.window_bounds = Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds { origin: Default::default(), size: size(px(1440.), px(900.)) }));
    let now = at(28, 17, 35);
    let (h, shell) = cx.update(|cx| {
        let mut made = None;
        let (h, _) = gpui_kit::open_window(opts, cx, |window, cx| {
            let shell = cx.new(|cx| {
                let mut s = Shell::new(Store::default(), Some(path), Box::new(move || now), window, cx);
                s.model.store.events = vec![];
                s.model.set_view(gpui_calendar::ui::model::View::Week);
                s
            });
            made = Some(shell.clone());
            cx.new(|_| Covered { shell, cover_w })
        })
        .unwrap();
        (h, made.unwrap())
    });
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}
