//! Headless screenshots of the shell: `cargo run --example shots` writes `artifacts/*.png`.
//! Needs GPUI's offscreen renderer (Metal on macOS).
#[cfg(target_os = "macos")]
fn main() {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use gpui_calendar::store::Store;
    use gpui_calendar::store::settings::Mode;
    use gpui_calendar::ui::Shell;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, Bounds, HeadlessAppContext, WindowBounds, WindowOptions, assets::Assets, px, size};

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts");
    std::fs::create_dir_all(dir).expect("create artifacts dir");
    // (file, mode, show the Appearance screen, busy month sample)
    for (file, mode, appearance, busy) in [
        ("shell-light", Mode::Light, false, false),
        ("shell-dark", Mode::Dark, false, false),
        ("appearance-light", Mode::Light, true, false),
        ("month-light", Mode::Light, false, true),
        ("month-dark", Mode::Dark, false, true),
    ] {
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(Assets),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        let mut store = Store::default();
        store.settings.appearance.mode = mode;
        if busy {
            store.events = busy_month();
        }
        let now = NaiveDate::from_ymd_opt(2026, 10, 28).expect("date").and_hms_opt(17, 35, 0).expect("time");
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: Default::default(), size: size(px(1440.), px(900.)) })),
            focus: false,
            show: false,
            ..gpui_calendar::ui::window_options()
        };
        let (handle, shell) = cx
            .update(|cx| gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| Shell::new(store, None, Box::new(move || now), window, cx))))
            .expect("open window");
        if appearance {
            cx.update(|cx| shell.update(cx, |s, cx| {
                s.model.toggle_appearance();
                s.sync(cx);
            }));
        }
        // The first frame lays the list out and applies the scroll to the month; the second draws it.
        for _ in 0..2 {
            cx.update_window(handle, |_, w, cx| w.render_frame(cx)).expect("render");
        }
        let image = cx.capture_screenshot(handle).expect("offscreen rendering");
        let path = format!("{dir}/{file}.png");
        image.save(&path).expect("save png");
        let (w, h) = image.dimensions();
        let first = *image.get_pixel(0, 0);
        assert!(image.pixels().any(|p| *p != first), "{file} is blank");
        println!("screenshot ok: {path} {w}x{h}");
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {}

/// Invented October 2026 events: a busy day (for "+n more"), bars across days and across a week edge.
#[cfg(target_os = "macos")]
fn busy_month() -> Vec<gpui_calendar::domain::event::Event> {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::{PERSONAL, SIDE, WORK};
    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).expect("date");
    let at = |d, h, m| day(d).and_hms_opt(h, m, 0).expect("time");
    let timed = |id, title, cal, d, h, m, mins| Event::timed(id, title, cal, at(d, h, m), at(d, h, m) + chrono::Duration::minutes(mins));
    vec![
        Event::all_day(1, "Offsite", WORK, day(5), day(7)),
        timed(2, "Standup", WORK, 5, 9, 30, 15),
        timed(3, "Pottery class", PERSONAL, 8, 18, 30, 90),
        Event::all_day(4, "Maya's birthday", PERSONAL, day(14), day(14)),
        timed(5, "Sprint planning", WORK, 14, 10, 0, 60),
        timed(6, "Dentist", PERSONAL, 14, 12, 30, 45),
        timed(7, "Review designs", WORK, 14, 14, 0, 60),
        timed(8, "Gym", PERSONAL, 14, 18, 0, 60),
        timed(9, "Release notes", SIDE, 14, 20, 0, 30),
        Event::all_day(10, "Hiking trip", PERSONAL, day(24), day(25)),
        timed(11, "Design review", WORK, 27, 14, 0, 90),
        timed(12, "Release checklist", WORK, 27, 14, 30, 30),
        timed(13, "Standup", WORK, 28, 9, 30, 15),
        timed(14, "Piano lesson", PERSONAL, 28, 18, 0, 60),
        Event::timed(15, "Late deploy window", WORK, at(30, 22, 0), at(31, 1, 0)),
        Event::all_day(16, "Conference", WORK, day(29), NaiveDate::from_ymd_opt(2026, 11, 2).expect("date")),
        timed(17, "Garden shed", SIDE, 31, 10, 0, 180),
    ]
}
