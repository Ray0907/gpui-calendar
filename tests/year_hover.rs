//! Year view pixels: a day, a padding day and a month title wash on hover (the state table's `wash-hover`, t1 at 6% over the
//! backdrop, never near-white), and the cursor ring is filled with the cell's own backdrop. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::{Rgb, contrast, over};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{MouseButton, point, px};
    use headless::*;

    let mut checks = 0;
    for mode in [Mode::Dark, Mode::Light] {
        let mut win = open(mode, Neutral::Cool, View::Year, vec![], (1440., 900.));
        let n = win.theme.neutral;
        // (id, backdrop): a weekday, a weekend, both paddings and a title.
        let cells = [("year-day-2026-10-07", n.surface), ("year-day-2026-10-10", n.subtle), ("year-pad-10-2026-11-03", n.surface), ("year-pad-10-2026-11-07", n.subtle), ("year-month-2026-10", n.surface)];
        for (id, backdrop) in cells {
            let b = win.bounds(id);
            // The cell's corner area holds no text: the date sits at the top centre, the dots under it.
            let probe = gpui_kit::Bounds { origin: point(b.left() + px(2.), b.bottom() - px(8.)), size: gpui_kit::size(b.size.width - px(4.), px(5.)) };
            let rest = mode_of(&win.shot().region(probe, 0.));
            assert!(delta(rest, backdrop) <= 3 || id.contains("month"), "{id} {mode:?}: rest {rest:?} vs backdrop {backdrop:?}");
            win.act(|w, cx| w.pointer_move(b.center() + point(px(0.), px(4.)), None, cx));
            let hot = mode_of(&win.shot().region(probe, 0.));
            win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
            let want = over(n.t1, 0.06, backdrop);
            let c = contrast(hot, backdrop);
            assert!(hot != rest && delta(hot, want) <= 3 && (1.05..=1.3).contains(&c) && (mode == Mode::Light || hot.luminance() <= 0.1), "{id} {mode:?}: hover {hot:?} rest {rest:?} want {want:?} contrast {c:.2}");
            checks += 1;
        }
        // The roving day on the weekend band: the ring's fill is the band, not the surface.
        win.act(|w, cx| {
            let pad = w.find("year-scroll").bounds().origin + point(px(4.), px(4.));
            w.pointer_down(pad, MouseButton::Left, cx);
            w.pointer_up(pad, MouseButton::Left, cx);
            ["right", "right", "right"].iter().for_each(|k| w.key_down(k, false, cx));
            w.pointer_move(point(px(2.), px(2.)), None, cx);
        });
        let b = win.bounds("year-ring-2026-10-31");
        let probe = gpui_kit::Bounds { origin: point(b.left() + px(3.), b.bottom() - px(8.)), size: gpui_kit::size(b.size.width - px(6.), px(5.)) };
        let fill = mode_of(&win.shot().region(probe, 0.));
        let want: Rgb = n.subtle;
        assert!(delta(fill, want) <= 3, "ring fill on a weekend cell {mode:?}: {fill:?} want {want:?}");
        checks += 1;
    }
    println!("year hover ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
