//! Mini-calendar selected day, light and dark: an ink ring around the cell and nothing else. The centre is the surface (or the hover wash, or the
//! accent on today), the ring sits 0.5px outside the cell edge in `ink`, and the number stays readable. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::{contrast, wash_hover_rgb};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{point, px};
    use headless::*;

    let mut checks = 0;
    for mode in [Mode::Light, Mode::Dark] {
        // (day to select, hovered, fill expected, label expected): the 28th is today in the fixture.
        for (day, hover) in [(20, false), (20, true), (28, false), (28, true)] {
            let mut win = open(mode, Neutral::Cool, View::Month, vec![], (1440., 900.));
            let id = format!("mini-day-2026-10-{day}");
            win.act(|w, cx| w.click(id.clone(), cx));
            win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
            let b = win.bounds(&id);
            if hover {
                win.act(|w, cx| w.pointer_move(b.center(), None, cx));
            }
            let (t, surface) = (win.theme, win.theme.neutral.surface);
            let (fill, label_want) = match (day == 28, hover) {
                (true, _) => (t.accent, t.accent_on),
                (false, false) => (surface, t.neutral.t1),
                (false, true) => (wash_hover_rgb(t.neutral.t1, surface), t.neutral.t1),
            };
            let shot = win.shot();
            let what = format!("{mode:?} day {day} hover {hover}");
            let got = mode_of(&shot.region(b, 3.));
            assert!(delta(got, fill) <= 3, "{what}: centre {got:?}, want {fill:?}");
            let (cx, cy) = (f32::from(b.center().x), f32::from(b.center().y));
            for (side, x, y) in [("left", f32::from(b.left()) - 0.5, cy), ("right", f32::from(b.right()) + 0.5, cy), ("top", cx, f32::from(b.top()) - 0.5), ("bottom", cx, f32::from(b.bottom()) + 0.5)] {
                let ring = shot.mean(x, y, 0.2);
                assert!(delta(ring, t.ink) <= 12, "{what}: ring {side} {ring:?}, want ink {:?}", t.ink);
                checks += 1;
            }
            let label = shot.region(b, 2.).into_iter().max_by_key(|&p| delta(p, got)).unwrap();
            assert!(delta(label, label_want) <= 24 && contrast(label, got) >= 4.5, "{what}: number {label:?} on {got:?}, want {label_want:?}");
            checks += 2;
        }
    }
    println!("mini selected ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
