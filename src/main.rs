use gpui_calendar::Spike;
use gpui_kit::{AppContext, WindowOptions};

fn main() {
    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Spike::new()))
            .expect("open window");
    });
}
