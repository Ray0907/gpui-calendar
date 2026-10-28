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
    // (file, mode, show the Appearance screen)
    for (file, mode, appearance) in [
        ("shell-light", Mode::Light, false),
        ("shell-dark", Mode::Dark, false),
        ("appearance-light", Mode::Light, true),
    ] {
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(Assets),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        let mut store = Store::default();
        store.settings.appearance.mode = mode;
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
        cx.update_window(handle, |_, w, cx| w.render_frame(cx)).expect("render");
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
