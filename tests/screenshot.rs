//! Needs GPUI's offscreen renderer (Metal on macOS); harness-less like gpui-kit's own rendering test.
#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::Spike;
    use gpui_kit::{AppContext, HeadlessAppContext, assets::Assets, px, size, test::TestWindowExt};
    use std::sync::Arc;
    let mut cx = HeadlessAppContext::with_platform(
        gpui_kit::platform::current_platform(true).text_system(),
        Arc::new(Assets),
        gpui_kit::platform::current_headless_renderer,
    );
    cx.update(gpui_kit::init);
    let (handle, _) = cx
        .update(|cx| {
            gpui_kit::open_window(
                gpui_kit::WindowOptions {
                    window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds {
                        origin: Default::default(),
                        size: size(px(320.), px(200.)),
                    })),
                    focus: false,
                    show: false,
                    ..Default::default()
                },
                cx,
                |_, cx| cx.new(|_| Spike::new()),
            )
        })
        .unwrap();
    cx.update_window(handle, |_, w, cx| w.render_frame(cx)).unwrap();
    let image = cx.capture_screenshot(handle).expect("offscreen rendering");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/target/spike");
    std::fs::create_dir_all(dir).unwrap();
    let path = format!("{dir}/spike.png");
    image.save(&path).unwrap();
    let (w, h) = image.dimensions();
    let first = *image.get_pixel(0, 0);
    assert!(w > 0 && h > 0);
    assert!(image.pixels().any(|p| *p != first), "screenshot is blank");
    println!("screenshot ok: {path} {w}x{h}");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
