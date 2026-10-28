//! Layout below the design width: window widths 1440 down to 360 for month, week, year and the event popover.
//! Writes artifacts/verify/narrow.txt and narrow-<w>-<view>.png for 800 and 480. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

/// (width, view) pairs that break today and are not fixed yet; the test fails if one starts passing or a new one breaks.
#[cfg(target_os = "macos")]
const KNOWN: &[(u32, &str)] = &[
    // The header (title, view switch, Today and the arrows) needs about 490px beside the rail; it needs a responsive layout to fit less.
    (480, "month"), (480, "week"), (480, "year"), (480, "popover"), (360, "month"), (360, "week"), (360, "year"), (360, "popover"),
];

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestQueryExt, TestWindowExt};
    use gpui_kit::{Bounds, Pixels, Role};
    use headless::*;

    let soon = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let events = || vec![Event::timed(12, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1))];
    let interactive = |r: Role| matches!(r, Role::Button | Role::CheckBox | Role::RadioButton | Role::TextInput | Role::MultilineTextInput | Role::ComboBox | Role::Switch);
    // Scroll content (grids, year rows) legitimately runs past the window vertically; everything else is chrome.
    let content = |id: &str| ["month-", "week-", "year-", "day-"].iter().any(|p| id.starts_with(p));
    let inside_x = |b: Bounds<Pixels>, w: f32| f32::from(b.left()) >= -0.5 && f32::from(b.right()) <= w + 0.5;
    let inside = |b: Bounds<Pixels>, w: f32| inside_x(b, w) && f32::from(b.top()) >= -0.5 && f32::from(b.bottom()) <= 900.5;
    let overlap = |a: Bounds<Pixels>, b: Bounds<Pixels>| {
        let (dx, dy) = (f32::from(a.right().min(b.right()) - a.left().max(b.left())), f32::from(a.bottom().min(b.bottom()) - a.top().max(b.top())));
        dx > 0.5 && dy > 0.5
    };
    let mut rows = vec![format!("{:<12} {:<8} {}", "width x view", "result", "first failing element and reason (failure count)")];
    let mut surprises = Vec::new();
    for w in [1440u32, 1280, 1024, 960, 800, 640, 480, 360] {
        for name in ["month", "week", "year", "popover"] {
            let view = match name { "week" => View::Week, "year" => View::Year, _ => View::Month };
            let mut win = open(Mode::Light, Neutral::Cool, view, events(), (w as f32, 900.));
            let wf = w as f32;
            let mut fails: Vec<String> = Vec::new();
            if name == "popover" {
                win.cx.update_window(win.h, |_, win, cx| win.render_frame(cx)).unwrap();
                win.cx.update_window(win.h, |_, window, cx| win.shell.update(cx, |s, cx| s.show_event(12, window, cx))).unwrap();
                win.render();
            }
            let els = win.cx.update_window(win.h, |_, window, _| window.elements().into_iter().filter_map(|e| Some((format!("{:?}", e.path().last()?).trim_start_matches("Name(\"").trim_end_matches("\")").to_string(), e.role(), e.bounds()))).collect::<Vec<_>>()).unwrap();
            for (id, role, b) in &els {
                if f32::from(b.size.width) > wf + 0.5 {
                    fails.push(format!("{id}: {:.0}px wide in a {w}px window", f32::from(b.size.width)));
                }
                if !role.is_some_and(interactive) {
                    continue;
                }
                let ok = if content(id) { inside_x(*b, wf) } else { inside(*b, wf) };
                if !ok {
                    fails.push(format!("{id}: bounds {:.0},{:.0} {:.0}x{:.0} leave the window", f32::from(b.origin.x), f32::from(b.origin.y), f32::from(b.size.width), f32::from(b.size.height)));
                }
            }
            let header: Vec<_> = els.iter().filter(|(id, ..)| id.starts_with("nav-") || id.starts_with("view-")).collect();
            for (i, (a, _, ab)) in header.iter().enumerate() {
                for (b, _, bb) in &header[i + 1..] {
                    if overlap(*ab, *bb) {
                        fails.push(format!("{a} overlaps {b}"));
                    }
                }
            }
            if name == "popover" {
                for id in ["pop-save", "pop-cancel"] {
                    match els.iter().find(|(i, ..)| i == id) {
                        None => fails.push(format!("{id}: not rendered")),
                        Some((_, _, b)) if !inside(*b, wf) => fails.push(format!("{id}: bounds leave the window")),
                        _ => {}
                    }
                }
                // Reachable: a real click on each one closes the card (a clean card saves without edits).
                for id in ["pop-cancel", "pop-save"] {
                    let mut again = open(Mode::Light, Neutral::Cool, View::Month, events(), (wf, 900.));
                    again.cx.update_window(again.h, |_, window, cx| again.shell.update(cx, |s, cx| s.show_event(12, window, cx))).unwrap();
                    again.render();
                    if again.cx.update_window(again.h, |_, window, _| window.try_find(id.to_string()).is_none()).unwrap() {
                        continue;
                    }
                    again.act(|window, cx| window.click(id.to_string(), cx));
                    if again.cx.update_window(again.h, |_, window, _| window.try_find(id.to_string()).is_some()).unwrap() {
                        fails.push(format!("{id}: a click did not reach it (card still open)"));
                    }
                }
            }
            if w == 800 || w == 480 {
                win.save_png(&format!("{}/narrow-{w}-{name}.png", artifact_dir()));
            }
            let known = KNOWN.contains(&(w, name));
            let result = if fails.is_empty() { "pass" } else { "FAIL" };
            if fails.is_empty() == known {
                surprises.push(format!("{w} x {name}: {}", if known { "now passes, remove from KNOWN".to_string() } else { fails[0].clone() }));
            }
            rows.push(format!("{:<12} {:<8} {}", format!("{w} x {name}"), if known && !fails.is_empty() { "known" } else { result }, fails.first().map_or(String::new(), |f| format!("{f} ({})", fails.len()))));
        }
    }
    // The title gives way first: from 800 to 640 it loses width while the view switch keeps its own and the arrows stay in the window.
    let header = |w: f32| {
        let mut win = open(Mode::Light, Neutral::Cool, View::Month, events(), (w, 900.));
        let (t, a, z, n) = (win.bounds("header-title"), win.bounds("view-day"), win.bounds("view-year"), win.bounds("nav-next"));
        (f32::from(t.size.width), f32::from(z.right() - a.left()), f32::from(n.right()))
    };
    let (wide, narrow) = (header(800.), header(640.));
    rows.push(format!("title shrinks first: title {:.0} -> {:.0}px, view switch {:.0} -> {:.0}px, last arrow right edge {:.0} of 640", wide.0, narrow.0, wide.1, narrow.1, narrow.2));
    if narrow.0 >= wide.0 - 50. || (narrow.1 - wide.1).abs() > 0.5 || narrow.2 > 640.5 {
        surprises.push(format!("title does not shrink first: 800px {wide:?}, 640px {narrow:?} (title, view switch, arrow right edge)"));
    }
    artifact("narrow.txt", &(rows.join("\n") + "\n"));
    assert!(surprises.is_empty(), "narrow layout surprises:\n{}", surprises.join("\n"));
    println!("narrow ok: {} width x view cases", rows.len() - 2);
}
#[cfg(not(target_os = "macos"))]
fn main() {}
