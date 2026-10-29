//! The popover's Title, both date fields and both time fields are the same box: FIELD_H (48px) tall, and the digits of a date and of a time
//! have the same size and baseline. Light and dark, windows 1440x900 and 960x600 (where the card must still fit). Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::ui::model::View;
    use gpui_kit::{Bounds, Pixels, point, px, size};
    use headless::*;

    const FIELD_H: f32 = 48.; // `popover/fields.rs`
    let start = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(14, 0, 0).unwrap();
    let event = Event::timed(1, "Piano lesson", WORK, start, start + chrono::Duration::hours(1));
    // The painted field: rows down the middle of the box, between the top and bottom outline (`border`), in logical px. The box's own
    // bounds are no proof: the date picker's wrapper is 48px whatever its button inside is.
    let painted = |shot: &Shot, b: Bounds<Pixels>, edge: gpui_calendar::theme::contrast::Rgb| {
        let x = f32::from(b.center().x);
        let rows: Vec<f32> = (0..(f32::from(b.size.height) + 12.) as usize * 2)
            .map(|i| f32::from(b.top()) - 6. + i as f32 / 2.)
            .filter(|&y| delta(shot.mean(x, y + 0.25, 0.25), edge) <= 10)
            .collect();
        (*rows.first().unwrap_or_else(|| panic!("no outline at x {x} in {b:?}; border {edge:?}, at top {:?}", shot.mean(x, f32::from(b.top()) + 0.5, 0.2))), *rows.last().unwrap() + 0.5)
    };
    // First and last row of ink (away from the fill) in the digits of the painted field, left of any icon.
    let ink = |shot: &Shot, top: f32, bottom: f32, b: Bounds<Pixels>| {
        let fill = mode_of(&shot.region(b, 4.));
        let rows: Vec<f32> = (0..((bottom - top - 8.) * 2.) as usize)
            .map(|i| top + 4. + i as f32 / 2.)
            .filter(|&y| shot.region(Bounds { origin: point(b.left() + px(14.), px(y)), size: size(px(46.), px(0.5)) }, 0.).iter().any(|&p| delta(p, fill) > 60))
            .collect();
        (*rows.first().expect("digits painted"), *rows.last().unwrap())
    };
    let mut checks = 0;
    for mode in [Mode::Light, Mode::Dark] {
        for (w, h) in [(1440., 900.), (960., 900.), (960., 600.)] {
            let mut win = open(mode, Neutral::Cool, View::Month, vec![event.clone()], (w, h));
            let shell = win.shell.clone();
            win.act(|win, cx| shell.update(cx, |s, cx| s.show_event(1, win, cx)));
            // The card fades in over 160 ms of wall clock.
            std::thread::sleep(std::time::Duration::from_millis(400));
            win.render();
            let card = win.bounds("popover");
            assert!(card.top() >= px(0.) && card.bottom() <= px(h), "{mode:?} {w}x{h}: popover {card:?} leaves the window");
            checks += 1;
            // At 600px the card's body scrolls: its lower fields are out of view, so only the fit is checked there.
            if h < 700. {
                continue;
            }
            win.act(|w, cx| w.blur(cx));
            let shot = win.shot();
            let edge = win.theme.neutral.border;
            let mut top_of = [(0., 0.); 5];
            for (n, id) in ["pop-title-box", "pop-start-date-box", "pop-end-date-box", "pop-start-time-box", "pop-end-time-box"].into_iter().enumerate() {
                let (top, bottom) = painted(&shot, win.bounds(id), edge);
                top_of[n] = (top, bottom);
                assert!((bottom - top - FIELD_H).abs() <= 1., "{mode:?} {w}x{h}: {id} is painted {} tall, want {FIELD_H}", bottom - top);
                checks += 1;
            }
            let (date_b, time_b) = (win.bounds("pop-start-date-box"), win.bounds("pop-start-time-box"));
            let date = ink(&shot, top_of[1].0, top_of[1].1, date_b);
            let time = ink(&shot, top_of[3].0, top_of[3].1, time_b);
            // Baseline: the bottom ink row relative to the painted top; size: the ink height.
            let (db, tb) = (date.1 - top_of[1].0, time.1 - top_of[3].0);
            assert!((db - tb).abs() <= 1., "{mode:?} {w}x{h}: digit baseline {db} in the date field, {tb} in the time field");
            let (dh, th) = (date.1 - date.0, time.1 - time.0);
            assert!((dh - th).abs() <= 1., "{mode:?} {w}x{h}: digit height {dh} in the date field, {th} in the time field");
            checks += 2;
        }
    }
    println!("field height ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
