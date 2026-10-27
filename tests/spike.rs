use gpui_calendar::Spike;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AppContext, MouseButton, TestAppContext, WindowOptions, WindowBounds, Bounds, point, px, size};

#[gpui_kit::test]
fn drag_on_cell_records_down_move_up(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (handle, view) = cx.update(|cx| {
        let (w, v) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(640.), px(480.)),
                })),
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(|_| Spike::new()),
        )
        .expect("open window");
        (w, v)
    });
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        let b = window.find("cell").bounds();
        assert_eq!(b.size.width, px(200.));
        assert_eq!(b.size.height, px(120.));
        let start = b.center();
        let end = start + point(px(30.), px(40.));
        window.pointer_down(start, MouseButton::Left, cx);
        window.pointer_move(end, Some(MouseButton::Left), cx);
        window.pointer_up(end, MouseButton::Left, cx);
    })
    .unwrap();
    cx.update(|cx| {
        let s = view.read(cx);
        let d = s.down.expect("down");
        let l = s.last.expect("move");
        let u = s.released.expect("up");
        assert_eq!(l - d, point(px(30.), px(40.)));
        assert_eq!(u, l);
    });
}
