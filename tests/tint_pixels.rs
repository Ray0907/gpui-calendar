//! Renderer tint blending: sampled pixels of the headless shell match what `theme::resolve` says, for every tint in both modes.
//! Writes artifacts/verify/tint.txt. Harness-less: GPUI's offscreen renderer is Metal.
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::store::{PERSONAL, SIDE, WORK};
    use gpui_calendar::theme::contrast::Rgb;
    use gpui_calendar::theme::palette::AVATAR;
    use gpui_kit::test::TestQueryExt;
    use headless::*;

    const TOL: u8 = 3;
    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
    let lerp = |a: Rgb, b: Rgb, t: f32| {
        let m = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
        Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
    };
    // One all-day chip per calendar, each on its own week, plus an event after "now" so the banner shows.
    let events = || {
        let soon = day(28).and_hms_opt(19, 0, 0).unwrap();
        vec![
            Event::all_day(10, "Work block", WORK, day(6), day(7)),
            Event::all_day(11, "Home block", PERSONAL, day(13), day(14)),
            Event::all_day(12, "Side block", SIDE, day(20), day(21)),
            Event::timed(13, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1)),
        ]
    };
    let mut rows = vec![format!("{:<14} {:<22} {:<14} {:<14} {:>5}  {}", "tint/mode", "sample", "expected", "got", "delta", "result")];
    let mut max = 0u8;
    let mut failures = Vec::new();
    for neutral in [Neutral::Cool, Neutral::Neutral, Neutral::Warm] {
        for mode in [Mode::Light, Mode::Dark] {
            let mut win = open(mode, neutral, gpui_calendar::ui::model::View::Month, events(), (1440., 900.));
            let t = win.theme;
            let mut samples: Vec<(String, Rgb, Rgb)> = Vec::new();
            let shot = win.shot();
            samples.push(("page background".into(), t.neutral.page, shot.at(2., 2.)));
            let sb = win.bounds("sidebar");
            samples.push(("sidebar surface".into(), t.neutral.surface, shot.at(f32::from(sb.left()) + 3., f32::from(sb.top() + sb.size.height * 0.56))));
            let bn = win.bounds("banner");
            let (bl, bw, by) = (f32::from(bn.left()), f32::from(bn.size.width), f32::from(bn.top() + bn.size.height * 0.5));
            for (name, off) in [("banner left edge", 2.), ("banner right edge", bw - 3.)] {
                // Gradient stop at the pixel centre, 90deg across the banner.
                samples.push((name.into(), lerp(t.banner.from, t.banner.to, (off + 0.5) / bw), shot.mean(bl + off.clamp(2., bw - 3.), by, 1.)));
            }
            for (id, cal, name) in [(10, WORK, "work"), (11, PERSONAL, "personal"), (12, SIDE, "side")] {
                let key = format!("month-ev-{id}-");
                let b = win.cx.update_window(win.h, |_, w, _| w.elements().into_iter().find(|e| e.path().last().is_some_and(|p| format!("{p:?}").contains(&key))).map(|e| e.bounds())).unwrap().expect("chip rendered");
                samples.push((format!("chip {name}"), t.cal(cal).unwrap().tint, mode_of(&shot.region(b, 3.))));
            }
            let av = win.bounds("avatar");
            let (ax, ay, aw) = (f32::from(av.left()), f32::from(av.top()), f32::from(av.size.width));
            // 135deg on a square: t = 0.5 + (dx + dy) / (2 * w); at 25%/75% along the diagonal both points sit inside the circle.
            for (name, f, tt) in [("avatar start", 0.25, 0.25), ("avatar mid", 0.5, 0.5), ("avatar end", 0.75, 0.75)] {
                samples.push((name.into(), lerp(AVATAR[0], AVATAR[1], tt), shot.mean(ax + aw * f, ay + aw * f, 1.5)));
            }
            let today = win.bounds("mini-day-2026-10-28");
            samples.push(("selected-day circle".into(), t.accent, mode_of(&shot.region(today, 6.))));
            for (name, want, got) in samples {
                let d = delta(want, got);
                max = max.max(d);
                let ok = d <= TOL;
                if !ok {
                    failures.push(format!("{neutral:?}/{mode:?} {name}: want {want:?} got {got:?} delta {d}"));
                }
                rows.push(format!("{:<14} {:<22} {:<14} {:<14} {:>5}  {}", format!("{neutral:?}/{mode:?}"), name, format!("{},{},{}", want.0, want.1, want.2), format!("{},{},{}", got.0, got.1, got.2), d, if ok { "pass" } else { "FAIL" }));
            }
        }
    }
    rows.push(format!("max delta {max} (tolerance {TOL}/255 per channel), {} failing", failures.len()));
    artifact("tint.txt", &(rows.join("\n") + "\n"));
    assert!(failures.is_empty(), "tint mismatches:\n{}", failures.join("\n"));
    println!("tint pixels ok: max delta {max}");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
