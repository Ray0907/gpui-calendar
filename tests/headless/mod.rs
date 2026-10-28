//! Shared helpers of the harness-less pixel tests: a headless shell, screenshots, sampling, artifact files.
#![allow(dead_code)]
pub mod states;
use std::sync::Arc;

use chrono::NaiveDate;
use gpui_calendar::domain::event::Event;
use gpui_calendar::store::Store;
use gpui_calendar::store::settings::{Mode, Neutral};
use gpui_calendar::theme::contrast::Rgb;
use gpui_calendar::theme::resolve::{Theme, resolve};
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Bounds, Entity, HeadlessAppContext, Pixels, WindowBounds, WindowOptions, px, size};

pub type Cx = HeadlessAppContext;

pub struct Win {
    pub cx: Cx,
    pub h: AnyWindowHandle,
    pub shell: Entity<Shell>,
    pub theme: Theme,
    pub w: f32,
}

pub fn open(mode: Mode, neutral: Neutral, view: View, events: Vec<Event>, (w, ht): (f32, f32)) -> Win {
    let mut cx = HeadlessAppContext::with_platform(
        gpui_kit::platform::current_platform(true).text_system(),
        Arc::new(gpui_calendar::assets()),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(gpui_kit::init);
    let mut store = Store::default();
    store.settings.appearance.mode = mode;
    store.settings.appearance.neutral = neutral;
    store.events = events;
    let theme = resolve(&store.settings.appearance, mode == Mode::Dark);
    let now = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(17, 35, 0).unwrap();
    let opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds { origin: Default::default(), size: size(px(w), px(ht)) })),
        focus: false,
        show: false,
        ..gpui_calendar::ui::window_options()
    };
    let (h, shell) = cx.update(|cx| gpui_kit::open_window(opts, cx, |win, cx| cx.new(|cx| Shell::new(store, None, Box::new(move || now), win, cx)))).unwrap();
    cx.update(|cx| {
        shell.update(cx, |s, cx| {
            s.model.set_view(view);
            s.sync(cx);
        })
    });
    let mut win = Win { cx, h, shell, theme, w };
    win.render();
    win
}

impl Win {
    pub fn render(&mut self) {
        for _ in 0..4 {
            self.cx.update_window(self.h, |_, w, cx| w.render_frame(cx)).unwrap();
        }
    }

    pub fn act(&mut self, f: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App)) {
        self.cx.update_window(self.h, |_, w, cx| f(w, cx)).unwrap();
        self.render();
    }

    pub fn bounds(&mut self, id: &str) -> Bounds<Pixels> {
        let id = id.to_string();
        self.cx.update_window(self.h, |_, w, _| w.find(id).bounds()).unwrap()
    }

    pub fn save_png(&mut self, path: &str) {
        self.cx.capture_screenshot(self.h).expect("offscreen rendering").save(path).unwrap();
    }

    pub fn shot(&mut self) -> Shot {
        let img = self.cx.capture_screenshot(self.h).expect("offscreen rendering");
        Shot { k: img.width() as f32 / self.w, width: img.width(), raw: img.as_raw().clone() }
    }
}

pub struct Shot {
    pub raw: Vec<u8>,
    pub width: u32,
    pub k: f32,
}

impl Shot {
    /// The pixel under logical point (x, y).
    pub fn at(&self, x: f32, y: f32) -> Rgb {
        self.px((x * self.k) as u32, (y * self.k) as u32)
    }

    /// The mean colour of the `r`-logical-px square around (x, y): gpui dithers gradients, so one pixel is noisy.
    pub fn mean(&self, x: f32, y: f32, r: f32) -> Rgb {
        let b = Bounds { origin: gpui_kit::point(px(x - r), px(y - r)), size: size(px(2. * r), px(2. * r)) };
        let all = self.region(b, 0.);
        let avg = |f: fn(&Rgb) -> u8| (all.iter().map(|p| u32::from(f(p))).sum::<u32>() as f32 / all.len() as f32).round() as u8;
        Rgb(avg(|p| p.0), avg(|p| p.1), avg(|p| p.2))
    }

    fn px(&self, x: u32, y: u32) -> Rgb {
        let i = ((y * self.width + x) * 4) as usize;
        Rgb(self.raw[i], self.raw[i + 1], self.raw[i + 2])
    }

    /// The pixels inside `b`, `inset` logical px in from its edge.
    pub fn region(&self, b: Bounds<Pixels>, inset: f32) -> Vec<Rgb> {
        let f = |p: Pixels, d: f32| ((f32::from(p) + d) * self.k) as u32;
        let (x0, x1, y0, y1) = (f(b.left(), inset), f(b.right(), -inset), f(b.top(), inset), f(b.bottom(), -inset));
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).map(|(x, y)| self.px(x, y)).collect()
    }
}

/// The most common colour: what the element is painted with.
pub fn mode_of(all: &[Rgb]) -> Rgb {
    let mut counts = std::collections::HashMap::new();
    for p in all {
        *counts.entry((p.0, p.1, p.2)).or_insert(0usize) += 1;
    }
    let (c, _) = counts.into_iter().max_by_key(|&(c, n)| (n, c)).unwrap();
    Rgb(c.0, c.1, c.2)
}

pub fn delta(a: Rgb, b: Rgb) -> u8 {
    a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)).max(a.2.abs_diff(b.2))
}

/// Write `body` to artifacts/verify/`name` (untracked, like artifacts/).
pub fn artifact(name: &str, body: &str) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/verify");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(format!("{dir}/{name}"), body).unwrap();
}

pub fn artifact_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts/verify")
}
