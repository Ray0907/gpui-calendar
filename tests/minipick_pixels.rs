//! Month picker pixels, light and dark, three tints: the card is `surface`; a month cell rests unfilled, hovers `wash-hover` (never white or near-white
//! in dark), presses at 85%, and a selected month has the 1px `ink` ring of a selected mini-month day; today's month is the `accent` fill; keyboard
//! focus shows the `ring` with the fill kept. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::{contrast, over, wash_hover_rgb};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{MouseButton, point, px};
    use headless::*;

    let away = point(px(2.), px(2.));
    let mut checks = 0;
    for (mode, tint) in [Mode::Light, Mode::Dark].into_iter().flat_map(|m| [Neutral::Cool, Neutral::Neutral, Neutral::Warm].map(|t| (m, t))) {
        let fresh = || {
            let mut win = open(mode, tint, View::Month, vec![], (1280., 900.));
            win.act(|w, cx| w.click("mini-title", cx));
            win.act(|w, cx| w.pointer_move(away, None, cx));
            win
        };
        let mut win = fresh();
        let (t, surface) = (win.theme, win.theme.neutral.surface);
        let what = format!("{mode:?} {tint:?}");

        // The card: `surface` inside, the `border` outline, the cells' text.
        let card = win.bounds("minipick");
        let shot = win.shot();
        let inside = mode_of(&shot.region(win.bounds("minipick-year"), 0.));
        assert!(delta(inside, surface) <= 3, "{what}: card fill {inside:?}, want surface {surface:?}");
        let edge = shot.mean(f32::from(card.left()) + 0.5, f32::from(card.center().y), 0.2);
        assert!(delta(edge, t.neutral.border) <= 6, "{what}: card outline {edge:?}, want border {:?}", t.neutral.border);
        checks += 2;

        // Plain month (March): rest, hover, pressed.
        let id = "minipick-month-2026-03";
        let b = win.bounds(id);
        let fill = |win: &mut Win| mode_of(&win.shot().region(b, 3.));
        let rest = fill(&mut win);
        assert!(delta(rest, surface) <= 3, "{what}: plain month rests unfilled, got {rest:?}");
        win.act(|w, cx| w.pointer_move(b.center(), None, cx));
        let hot = fill(&mut win);
        let want = wash_hover_rgb(t.neutral.t1, surface);
        let c = contrast(hot, surface);
        assert!(delta(hot, want) <= 3 && (mode == Mode::Light || hot.luminance() <= 0.1) && (1.08..=1.25).contains(&c), "{what}: hover {hot:?}, want wash-hover {want:?}, contrast {c:.2}");
        win.act(|w, cx| w.pointer_down(b.center(), MouseButton::Left, cx));
        let held = fill(&mut win);
        let want = over(want, 0.85, surface);
        assert!(delta(held, want) <= 3 && delta(held, hot) > 0, "{what}: pressed {held:?}, want the hover at 85% {want:?}");
        win.act(|w, cx| w.pointer_up(away, MouseButton::Left, cx));
        checks += 3;

        // Selected (the view's month, October, which is also today): the accent fill stays, the ink ring goes round it, the label is `accent_on`; hover changes nothing.
        let oct = win.bounds("minipick-month-2026-10");
        for hover in [false, true] {
            win.act(|w, cx| w.pointer_move(if hover { oct.center() } else { away }, None, cx));
            let shot = win.shot();
            let centre = mode_of(&shot.region(oct, 3.));
            assert!(delta(centre, t.accent) <= 3, "{what} today hover {hover}: fill {centre:?}, want accent {:?}", t.accent);
            let (cx, cy) = (f32::from(oct.center().x), f32::from(oct.center().y));
            for (side, x, y) in [("left", f32::from(oct.left()) - 0.5, cy), ("right", f32::from(oct.right()) + 0.5, cy), ("top", cx, f32::from(oct.top()) - 0.5), ("bottom", cx, f32::from(oct.bottom()) + 0.5)] {
                let ring = shot.mean(x, y, 0.5);
                assert!(delta(ring, t.ink) <= 12, "{what} today hover {hover}: ring {side} {ring:?}, want ink {:?}", t.ink);
                checks += 1;
            }
            let label = shot.region(oct, 2.).into_iter().max_by_key(|&p| delta(p, centre)).unwrap();
            assert!(delta(label, t.accent_on) <= 24, "{what}: label {label:?}, want accent_on {:?}", t.accent_on);
            checks += 2;
        }

        // Selected but not today: another day of the view's month is not possible, so pick September in the mini calendar and the view moves with it.
        win.act(|w, cx| w.click("minipick-month-2026-09", cx));
        win.act(|w, cx| w.click("mini-title", cx));
        win.act(|w, cx| w.pointer_move(away, None, cx));
        let sep = win.bounds("minipick-month-2026-09");
        let shot = win.shot();
        let centre = mode_of(&shot.region(sep, 3.));
        assert!(delta(centre, surface) <= 3, "{what}: selected month centre {centre:?}, want surface {surface:?}");
        let (cx, cy) = (f32::from(sep.center().x), f32::from(sep.center().y));
        for (side, x, y) in [("left", f32::from(sep.left()) - 0.5, cy), ("right", f32::from(sep.right()) + 0.5, cy), ("top", cx, f32::from(sep.top()) - 0.5), ("bottom", cx, f32::from(sep.bottom()) + 0.5)] {
            let ring = shot.mean(x, y, 0.5);
            assert!(delta(ring, t.ink) <= 12, "{what}: selected ring {side} {ring:?}, want ink {:?}", t.ink);
            checks += 1;
        }
        let label = shot.region(sep, 2.).into_iter().max_by_key(|&p| delta(p, centre)).unwrap();
        assert!(delta(label, t.neutral.t1) <= 24 && contrast(label, centre) >= 4.5, "{what}: selected label {label:?}, want t1");
        win.act(|w, cx| w.pointer_move(sep.center(), None, cx));
        let hot = mode_of(&win.shot().region(sep, 3.));
        assert!(delta(hot, wash_hover_rgb(t.neutral.t1, surface)) <= 3, "{what}: a selected month still washes on hover, got {hot:?}");
        checks += 3;

        // Keyboard focus: the picker opens from the title with Enter, then Left moves to a plain month: the ring shows, the fill is the rest value, and hovering it changes nothing under the ring.
        let mut win = fresh();
        let title = win.cx.update(|cx| win.shell.read(cx).minipick_title_focus());
        win.act(|w, cx| w.press("escape", cx));
        win.act(|w, cx| title.focus(w, cx));
        win.act(|w, cx| w.press("enter", cx));
        win.act(|w, cx| w.press("left", cx));
        let sep = win.bounds("minipick-month-2026-09");
        let shot = win.shot();
        let ring = shot.mean(f32::from(sep.left()) - 3., f32::from(sep.center().y), 0.4);
        assert!(delta(ring, t.ring) <= 6, "{what}: focus ring {ring:?}, want {:?}", t.ring);
        let centre = mode_of(&shot.region(sep, 3.));
        assert!(delta(centre, surface) <= 3, "{what}: focused month keeps its rest fill, got {centre:?}");
        checks += 2;
    }
    println!("minipick pixels ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
