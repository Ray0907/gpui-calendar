//! Date picker pixels, light and dark, three tints: a hovered day and a hovered arrow wash with `wash-hover` over the popup surface
//! (`secondary_hover`, written by `token_map`), never the library's gray or a near-white. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::domain::form::Field;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::{contrast, wash_hover_rgb};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{Bounds, point, px, size};
    use headless::*;

    let start = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let event = Event::timed(1, "Piano lesson", WORK, start, start + chrono::Duration::hours(1));
    let mut checks = 0;
    for (mode, tint) in [Mode::Light, Mode::Dark].into_iter().flat_map(|m| [Neutral::Cool, Neutral::Neutral, Neutral::Warm].map(|t| (m, t))) {
        let mut win = open(mode, tint, View::Month, vec![event.clone()], (1280., 900.));
        let shell = win.shell.clone();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx)));
        let picker = win.cx.update(|cx| win.shell.read(cx).popover_date_id(Field::StartDate)).expect("a date picker");
        win.act(|w, cx| w.click(picker.clone(), cx));
        // The card fades in over 160 ms of wall clock (`popover/view.rs` FADE_MS); sampled mid-fade, light white reads 251.
        std::thread::sleep(std::time::Duration::from_millis(400));
        win.render();
        let surface = win.theme.neutral.surface;
        // Enabled in-month days (the 20th, and today, the 28th: the picker paints today with no fill, rest = `surface`, not the `subtle` of the doc's old row) and both arrows. The probe is a strip on the cell's left edge, clear of its number.
        let find = |win: &mut Win, d: u32| (0..6).flat_map(|r| (0..7).map(move |c| format!("calendar-2026-10-{d}-{c}-{r}"))).find(|id| win.cx.update_window(win.h, |_, w, _| w.try_find(id.clone()).is_some()).unwrap()).unwrap_or_else(|| panic!("Oct {d} in the calendar"));
        let (day, today) = (find(&mut win, 20), find(&mut win, 28));
        for (id, backdrop) in [(day.as_str(), surface), (today.as_str(), surface), ("calendar-prev", surface), ("calendar-next", surface)] {
            let snap = win.cx.update_window(win.h, |_, w, _| w.find(id.to_string())).unwrap();
            assert!(snap.visible() && snap.disabled() != Some(true), "{id} {mode:?} {tint:?}: not an enabled control ({snap:?})");
            let b = win.bounds(id);
            let probe = Bounds { origin: point(b.left() + px(2.), b.center().y - px(3.)), size: size(px(3.), px(6.)) };
            let rest = mode_of(&win.shot().region(probe, 0.));
            win.act(|w, cx| w.pointer_move(b.center(), None, cx));
            let hot = mode_of(&win.shot().region(probe, 0.));
            win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
            let want = wash_hover_rgb(win.theme.neutral.t1, backdrop);
            let c = contrast(hot, backdrop);
            assert!(delta(rest, backdrop) <= 3, "{id} {mode:?} {tint:?}: rest {rest:?} is not {backdrop:?}");
            assert!(delta(hot, want) <= 3 && (mode == Mode::Light || hot.luminance() <= 0.1) && (1.08..=1.25).contains(&c), "{id} {mode:?} {tint:?}: hover {hot:?} want wash-hover {want:?} over {backdrop:?}, contrast {c:.2}");
            checks += 1;
        }
        // The rest of the card: its fill is `surface`, the picked day (Oct 14, the event's start) is `ink` with an `on_ink` label,
        // and an arrow's icon rests in `t3`. A label or icon is thin: its darkest-to-lightest pixel is the role colour within 24.
        let far = |shot: &Shot, b: Bounds<gpui_kit::Pixels>, from: gpui_calendar::theme::contrast::Rgb| shot.region(b, 2.).into_iter().max_by_key(|&p| delta(p, from)).unwrap();
        let (prev, id14) = (win.bounds("calendar-prev"), find(&mut win, 14));
        let picked = win.bounds(&id14);
        let shot = win.shot();
        let card = mode_of(&shot.region(Bounds { origin: point(prev.left() - px(10.), prev.center().y - px(3.)), size: size(px(3.), px(6.)) }, 0.));
        let fill = mode_of(&shot.region(picked, 3.));
        let label = far(&shot, picked, fill);
        assert!(delta(card, surface) <= 3, "{mode:?} {tint:?}: card fill {card:?}, want surface {surface:?}");
        assert!(delta(fill, win.theme.ink) <= 3, "{mode:?} {tint:?}: picked day {fill:?}, want ink {:?}", win.theme.ink);
        assert!(delta(label, win.theme.on_ink) <= 24, "{mode:?} {tint:?}: picked day label {label:?}, want on_ink {:?}", win.theme.on_ink);
        checks += 3;
    }
    println!("date hover ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
