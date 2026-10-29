//! The open time list's pixels, light and dark, three tints: a hovered row is ONE `wash-hover` over the list's `surface`, the selected row keeps its
//! `ink` fill under the pointer (nothing stacked on it), no neighbour of a hovered row changes, and a pressed-down row is `wash-press`. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::{contrast, over, wash_hover_rgb};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{Bounds, MouseButton, point, px, size};
    use headless::*;

    let start = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let event = Event::timed(1, "Piano lesson", WORK, start, start + chrono::Duration::hours(1));
    let mut checks = 0;
    for (mode, tint) in [Mode::Light, Mode::Dark].into_iter().flat_map(|m| [Neutral::Cool, Neutral::Neutral, Neutral::Warm].map(|t| (m, t))) {
        let mut win = open(mode, tint, View::Month, vec![event.clone()], (1280., 900.));
        let shell = win.shell.clone();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx)));
        std::thread::sleep(std::time::Duration::from_millis(400)); // the card's 160 ms fade
        win.render();
        win.act(|w, cx| w.click("pop-start-time-trigger", cx));
        win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
        let (surface, n) = (win.theme.neutral.surface, win.theme.neutral);
        let what = format!("{mode:?} {tint:?}");
        // A strip on the row's left edge, clear of its label.
        let strip = |b: Bounds<gpui_kit::Pixels>| Bounds { origin: point(b.left() + px(2.), b.center().y - px(3.)), size: size(px(3.), px(6.)) };
        let (sel, below, below2) = (win.bounds("pop-start-time-opt-1900"), win.bounds("pop-start-time-opt-1915"), win.bounds("pop-start-time-opt-1930"));
        if let Ok(dir) = std::env::var("SHOT_DIR") {
            win.save_png(&format!("{dir}/time-list-{mode:?}.png"));
        }
        let rest = win.shot();
        assert!(delta(mode_of(&rest.region(strip(below), 0.)), surface) <= 3, "{what}: a row at rest is the list's surface");
        assert!(delta(mode_of(&rest.region(strip(sel), 0.)), win.theme.ink) <= 3, "{what}: the selected row is ink");
        // Hover a plain row: one wash over the surface; the selected row above and the next row below do not change.
        win.act(|w, cx| w.pointer_move(below.center(), None, cx));
        let hot = win.shot();
        let (got, want) = (mode_of(&hot.region(strip(below), 0.)), wash_hover_rgb(n.t1, surface));
        assert!(delta(got, want) <= 3 && (1.08..=1.25).contains(&contrast(got, surface)), "{what}: hovered row {got:?}, want wash-hover {want:?}");
        assert!(delta(mode_of(&hot.region(strip(sel), 0.)), win.theme.ink) <= 3, "{what}: the selected row beside a hovered one keeps ink");
        assert!(delta(mode_of(&hot.region(strip(below2), 0.)), surface) <= 3, "{what}: the row after a hovered one stays at rest");
        // Hold the pointer down: wash-press, a fill and not a dim.
        win.act(|w, cx| w.pointer_down(below.center(), MouseButton::Left, cx));
        let held = mode_of(&win.shot().region(strip(below), 0.));
        let want = over(n.t1, 0.10, surface);
        assert!(delta(held, want) <= 3, "{what}: pressed row {held:?}, want wash-press {want:?}");
        win.act(|w, cx| w.pointer_up(point(px(2.), px(2.)), MouseButton::Left, cx));
        // Hover the selected row: still the one ink fill.
        win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
        win.act(|w, cx| w.pointer_move(sel.center(), None, cx));
        let on_sel = mode_of(&win.shot().region(strip(sel), 0.));
        assert!(delta(on_sel, win.theme.ink) <= 3, "{what}: hovered selected row {on_sel:?}, want ink {:?}", win.theme.ink);
        checks += 8;
    }
    println!("time list hover ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
