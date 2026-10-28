//! Headless screenshots of the shell: `cargo run --example shots` writes `artifacts/*.png`.
//! Needs GPUI's offscreen renderer (Metal on macOS).
#[cfg(target_os = "macos")]
fn main() {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use gpui_calendar::store::Store;
    use gpui_calendar::store::settings::Mode;
    use gpui_calendar::ui::Shell;
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, Bounds, HeadlessAppContext, WindowBounds, WindowOptions, assets::Assets, px, size};

    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/artifacts");
    std::fs::create_dir_all(dir).expect("create artifacts dir");
    // (file, mode, show the Appearance screen, view, sample events, clock)
    for (file, mode, appearance, view, sample, (h, m)) in [
        ("shell-light", Mode::Light, false, View::Month, Sample::Seed, (17, 35)),
        ("shell-dark", Mode::Dark, false, View::Month, Sample::Seed, (17, 35)),
        ("appearance-light", Mode::Light, true, View::Month, Sample::Seed, (17, 35)),
        ("month-light", Mode::Light, false, View::Month, Sample::Month, (17, 35)),
        ("month-dark", Mode::Dark, false, View::Month, Sample::Month, (17, 35)),
        ("week-light", Mode::Light, false, View::Week, Sample::Week, (11, 20)),
        ("week-dark", Mode::Dark, false, View::Week, Sample::Week, (11, 20)),
        ("day-light", Mode::Light, false, View::Day, Sample::Week, (11, 20)),
        ("day-dark", Mode::Dark, false, View::Day, Sample::Week, (11, 20)),
        ("year-light", Mode::Light, false, View::Year, Sample::Year, (17, 35)),
        ("year-dark", Mode::Dark, false, View::Year, Sample::Year, (17, 35)),
    ] {
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(Assets),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        let mut store = Store::default();
        store.settings.appearance.mode = mode;
        match sample {
            Sample::Seed => {}
            Sample::Month => store.events = busy_month(),
            Sample::Week => store.events = busy_week(),
            Sample::Year => store.events = busy_year(),
        }
        let now = NaiveDate::from_ymd_opt(2026, 10, 28).expect("date").and_hms_opt(h, m, 0).expect("time");
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: Default::default(), size: size(px(1440.), px(900.)) })),
            focus: false,
            show: false,
            ..gpui_calendar::ui::window_options()
        };
        let (handle, shell) = cx
            .update(|cx| gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| Shell::new(store, None, Box::new(move || now), window, cx))))
            .expect("open window");
        cx.update(|cx| shell.update(cx, |s, cx| {
            s.model.set_view(view);
            if appearance {
                s.model.toggle_appearance();
            }
            s.sync(cx);
        }));
        // The first frame lays the list out and applies the scroll to the month or the grid; the next draw it.
        for _ in 0..4 {
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

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum Sample {
    Seed,
    Month,
    Week,
    Year,
}

/// Invented week of Oct 26: overlaps, a third overlap (the "+1 more" block), a bar across two days and a pill.
#[cfg(target_os = "macos")]
fn busy_week() -> Vec<gpui_calendar::domain::event::Event> {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::{PERSONAL, SIDE, WORK};
    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).expect("date");
    let timed = |id, title, cal, d, h, m, mins| {
        let start = day(d).and_hms_opt(h, m, 0).expect("time");
        Event::timed(id, title, cal, start, start + chrono::Duration::minutes(mins))
    };
    vec![
        Event::all_day(1, "Offsite", WORK, day(27), day(28)),
        Event::all_day(2, "Anna's birthday", PERSONAL, day(31), day(31)),
        timed(3, "Daily stand-up", WORK, 26, 9, 30, 30),
        timed(4, "Sprint planning", WORK, 26, 10, 0, 90),
        timed(5, "Focus time", SIDE, 27, 9, 0, 120),
        timed(6, "Stand-up", WORK, 27, 9, 30, 15),
        timed(7, "Code review", PERSONAL, 27, 10, 0, 60),
        timed(8, "Planning notes", WORK, 27, 10, 15, 30),
        timed(9, "Product sync", WORK, 27, 13, 0, 60),
        timed(10, "Marketing", WORK, 27, 13, 45, 90),
        timed(11, "Daily stand-up", WORK, 28, 9, 30, 30),
        timed(12, "Design review", WORK, 28, 14, 0, 60),
        timed(13, "Daily stand-up", WORK, 29, 9, 30, 30),
        timed(14, "1:1 with Sam", WORK, 29, 11, 0, 30),
        timed(15, "Lunch with Maria", PERSONAL, 29, 12, 30, 60),
        timed(16, "Sprint retro", WORK, 30, 16, 0, 45),
        timed(17, "Ship weekly update", SIDE, 30, 16, 30, 30),
        timed(18, "Long run", PERSONAL, 31, 8, 0, 60),
        timed(19, "Water plants", PERSONAL, 31, 10, 0, 30),
    ]
}

/// October's events plus invented ones across the other months: single dots, all three shapes on one day, a bar over New Year.
#[cfg(target_os = "macos")]
fn busy_year() -> Vec<gpui_calendar::domain::event::Event> {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::{PERSONAL, SIDE, WORK};
    let day = |m, d| NaiveDate::from_ymd_opt(2026, m, d).expect("date");
    let timed = |id, title, cal, m, d, h| {
        let start = day(m, d).and_hms_opt(h, 0, 0).expect("time");
        Event::timed(id, title, cal, start, start + chrono::Duration::minutes(60))
    };
    let mut events = busy_month();
    events.extend([
        timed(101, "Kickoff", WORK, 1, 6, 10),
        timed(102, "Ski day", PERSONAL, 2, 14, 9),
        Event::all_day(103, "Spring break", PERSONAL, day(3, 30), day(4, 3)),
        timed(104, "Demo", SIDE, 5, 12, 15),
        timed(105, "Budget review", WORK, 5, 12, 16),
        timed(106, "Birthday dinner", PERSONAL, 5, 12, 19),
        timed(107, "Planning", WORK, 6, 2, 9),
        Event::all_day(108, "Summer trip", PERSONAL, day(7, 20), day(7, 27)),
        timed(109, "Launch", WORK, 9, 8, 11),
        timed(110, "Workshop", SIDE, 11, 17, 18),
        Event::all_day(111, "Holiday party", PERSONAL, day(12, 24), NaiveDate::from_ymd_opt(2026, 12, 25).expect("date")),
    ]);
    events
}
