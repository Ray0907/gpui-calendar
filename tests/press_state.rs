//! Press state, not feel: with the pointer held down the control's pixels are the documented 85% of its own over what is behind it,
//! and they return to the unpressed ones after release. Writes artifacts/verify/press.txt. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::store::WORK;
    use gpui_calendar::theme::contrast::Rgb;
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
    use gpui_kit::{Bounds, MouseButton, Pixels, point, px, size};
    use headless::*;

    const DOCUMENTED: f32 = 0.85;
    const TOL: u8 = 3;
    const MIN: usize = 20;
    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
    let events = || {
        let (s, soon) = (day(14).and_hms_opt(10, 0, 0).unwrap(), day(28).and_hms_opt(19, 0, 0).unwrap());
        vec![
            Event::all_day(10, "Work block", WORK, day(6), day(7)),
            Event::timed(11, "Standup", WORK, s, s + chrono::Duration::minutes(30)),
            Event::timed(12, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1)),
        ]
    };
    let chip = |win: &mut Win, id: u32| {
        let key = format!("month-ev-{id}-");
        win.cx.update_window(win.h, |_, w, _| w.elements().into_iter().find(|e| e.path().last().is_some_and(|p| format!("{p:?}").contains(&key))).map(|e| e.bounds())).unwrap().expect("chip rendered")
    };
    let mut rows = vec![format!("{:<14} {:<22} {:>14} {:>16} {:>14} {:>8}  {}", "mode", "element", "median ratio", "max vs 0.85 pred", "release delta", "samples", "result")];
    let mut failures = Vec::new();
    for mode in [Mode::Light, Mode::Dark] {
        for name in ["nav-next", "view-week", "month chip (all-day)", "month chip (timed)", "month chip (focused)", "popover Save"] {
            let mut win = open(mode, Neutral::Cool, View::Month, events(), (1440., 900.));
            let surface = win.theme.neutral.surface;
            let (b, backdrop) = match name {
                "nav-next" => (win.bounds("nav-next"), None),
                "view-week" => (win.bounds("view-week"), Some(win.theme.neutral.field)),
                "month chip (all-day)" => (chip(&mut win, 10), Some(surface)),
                "month chip (timed)" => (chip(&mut win, 11), Some(surface)),
                "month chip (focused)" => {
                    // Cell click focuses the grid, Right twice moves the roving focus onto the Standup chip: it carries the ring shadow.
                    win.act(|w, cx| {
                        w.click("month-cell-2026-10-14", cx);
                        // Reading order: Work block, then Standup.
                        w.press("right", cx);
                        w.press("right", cx);
                    });
                    (chip(&mut win, 11), Some(surface))
                }
                _ => {
                    win.cx.update_window(win.h, |_, w, cx| win.shell.update(cx, |s, cx| s.show_event(12, w, cx))).unwrap();
                    win.render();
                    (win.bounds("pop-save"), None)
                }
            };
            let before = win.shot();
            let backdrop = backdrop.unwrap_or_else(|| before.at(f32::from(b.left()) - 4., f32::from(b.top() + b.size.height * 0.5)));
            let (u, c) = (before.region(b, 2.), b.center());
            win.act(|w, cx| w.pointer_down(c, MouseButton::Left, cx));
            let held_shot = win.shot();
            let held = held_shot.region(b, 2.);
            // Ratios come from the interior only: the focused chip's ring band hugs the edge and vanishes on press, which is ring_ok's job, not the 85%.
            let inner = if name.contains("focused") { 5. } else { 2. };
            let (u_in, held_in) = (before.region(b, inner), held_shot.region(b, inner));
            // The 2px band just outside the chip is where its ring shadow shows. Count pixels that are closer to the ring than to what is behind.
            let ring = win.theme.ring;
            let strip = |x: Pixels| Bounds { origin: point(x, b.top() + px(4.)), size: size(px(3.), b.size.height - px(8.)) };
            let ringed = |s: &Shot| [strip(b.left() - px(4.)), strip(b.right() + px(1.))].into_iter().flat_map(|r| s.region(r, 0.)).filter(|&p| delta(p, backdrop) >= 40 && delta(p, ring) < delta(p, backdrop)).count();
            let (ring_before, ring_held) = (ringed(&before), ringed(&held_shot));
            // The press starts a move drag, but a drag shows nothing until the pointer reaches another cell: the held shot is :active only.
            let starts = |win: &mut Win| win.cx.update(|cx| win.shell.read(cx).model.store.events.iter().map(|e| e.start).collect::<Vec<_>>());
            let moved_before = starts(&mut win);
            win.act(|w, cx| w.pointer_up(point(px(2.), px(2.)), MouseButton::Left, cx));
            let after = win.shot().region(b, 2.);
            // Per pixel: what 85% of the unpressed pixel over the backdrop would be; and the ratio where the pixel differs from the backdrop.
            let pred = |u: Rgb| {
                let m = |u: u8, b: u8| (f32::from(u) * DOCUMENTED + f32::from(b) * (1. - DOCUMENTED)).round() as u8;
                Rgb(m(u.0, backdrop.0), m(u.1, backdrop.1), m(u.2, backdrop.2))
            };
            // Flat pixels only (the element's own fill, or what is behind it): GPUI applies opacity per primitive, so glyph edges are not a clean 85% blend.
            let fill = mode_of(&u);
            let flat = |p: &Rgb| *p == fill || *p == backdrop;
            let max_pred = u.iter().zip(&held).filter(|(u, _)| flat(u)).map(|(&u, &h)| delta(pred(u), h)).max().unwrap();
            let release = u.iter().zip(&after).map(|(&u, &r)| delta(u, r)).max().unwrap();
            let mut ratios: Vec<f32> = u_in
                .iter()
                .zip(&held_in)
                .flat_map(|(u, h)| [(u.0, h.0, backdrop.0), (u.1, h.1, backdrop.1), (u.2, h.2, backdrop.2)])
                .filter(|&(u, _, bk)| u.abs_diff(bk) >= 40)
                .map(|(u, h, bk)| (f32::from(h) - f32::from(bk)) / (f32::from(u) - f32::from(bk)))
                .collect();
            ratios.sort_by(|a, b| a.total_cmp(b));
            let median = ratios.get(ratios.len() / 2).copied();
            // A control with too few contrasting samples proves nothing; and a press must change at least one pixel.
            let changed = u.iter().zip(&held).any(|(u, h)| u != h);
            let focused = name.contains("focused");
            let ring_ok = !focused || (ring_before >= MIN && ring_held * 10 <= ring_before);
            let drag_idle = moved_before == starts(&mut win);
            let ok = max_pred <= TOL && release <= TOL && ratios.len() >= MIN && median.is_some_and(|m| (m - DOCUMENTED).abs() <= 0.02) && changed && ring_ok && drag_idle;
            if !ok {
                failures.push(format!("{mode:?} {name}: median {median:?} of {} samples, changed {changed}, ring px {ring_before} -> {ring_held}, max vs pred {max_pred}, release {release}, events untouched {drag_idle}", ratios.len()));
            }
            rows.push(format!("{:<14} {:<22} {:>14} {:>16} {:>14} {:>8}  {}", format!("{mode:?}"), name, median.map_or("none".into(), |m| format!("{m:.3}")), max_pred, release, ratios.len(), if ok { "pass" } else { "FAIL" }));
            if focused {
                rows.push(format!("{:<14} ring pixels around the focused chip: {ring_before} unpressed, {ring_held} held", ""));
            }
        }
    }
    rows.push(format!("documented ratio {DOCUMENTED}; tolerance {TOL}/255 per channel, ratio within 0.02; {} failing", failures.len()));
    artifact("press.txt", &(rows.join("\n") + "\n"));
    assert!(failures.is_empty(), "press state mismatches:\n{}", failures.join("\n"));
    println!("press state ok: {} rows", rows.len() - 2);
}
#[cfg(not(target_os = "macos"))]
fn main() {}
