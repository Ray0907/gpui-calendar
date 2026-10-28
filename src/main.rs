use gpui_calendar::store;
use gpui_calendar::ui::{Shell, local_clock, window_options};
use gpui_kit::AppContext;

fn main() {
    gpui_kit::application().with_assets(gpui_calendar::assets()).run(move |cx| {
        gpui_kit::init(cx);
        // A file that cannot be read is kept as it is: defaults on screen, saving off, reason shown.
        gpui_kit::open_window(window_options(), cx, |window, cx| cx.new(|cx| Shell::start(store::default_path(), local_clock(), window, cx)))
            .expect("open window");
    });
}
