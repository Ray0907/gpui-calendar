//! Time field pixels, light and dark: it rests on the Title field's fill, the focused segment is `selection` (ink at 25%) over that fill,
//! hover redraws the outline in `t2`, an unreadable draft time is invalid (danger outline) and a disabled field is the same field at 50%. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::domain::form::Field;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::Rgb;
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{point, px};
    use headless::*;

    let start = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let event = Event::timed(1, "Piano lesson", WORK, start, start + chrono::Duration::hours(1));
    let blend = |over: Rgb, under: Rgb, a: f32| {
        let f = |o: u8, u: u8| (f32::from(o) * a + f32::from(u) * (1. - a)).round() as u8;
        Rgb(f(over.0, under.0), f(over.1, under.1), f(over.2, under.2))
    };
    let mut checks = 0;
    for mode in [Mode::Light, Mode::Dark] {
        let mut win = open(mode, Neutral::Cool, View::Month, vec![event.clone()], (1280., 900.));
        let shell = win.shell.clone();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx)));
        // The card fades in over 160 ms of wall clock (`popover/view.rs` FADE_MS).
        std::thread::sleep(std::time::Duration::from_millis(400));
        win.render();
        let n = win.theme.neutral;
        let id = win.cx.update(|cx| win.shell.read(cx).popover_time_id(Field::StartTime)).expect("a time field");
        // The box around the field, frame and all: the field's own id is on the content inside its 1px outline.
        let field = win.bounds("pop-start-time-box");
        let title = win.bounds("pop-title");
        // Rest: the fill is the Title field's, probed in the padding strip left of the first segment / the text.
        let fill = |win: &mut Win, b: gpui_kit::Bounds<gpui_kit::Pixels>| mode_of(&win.shot().region(gpui_kit::Bounds { origin: point(b.right() - px(14.), b.top() + px(8.)), size: gpui_kit::size(px(8.), px(32.)) }, 0.));
        let (rest, want) = (fill(&mut win, field), fill(&mut win, title));
        assert!(delta(rest, want) <= 3, "{mode:?}: time field fill {rest:?}, Title field fill {want:?}");
        checks += 1;
        // Outline at rest is `border`, probed at the middle of the top edge.
        let edge = |win: &mut Win| win.shot().mean(field.center().x.into(), f32::from(field.top()) + 0.5, 0.4);
        let at_rest = edge(&mut win);
        assert!(delta(at_rest, n.border) <= 8, "{mode:?}: outline at rest {at_rest:?}, want border {:?}", n.border);
        // Hover: the outline is `t2`.
        win.act(|w, cx| w.pointer_move(field.center(), None, cx));
        let hot = edge(&mut win);
        win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
        assert!(delta(hot, n.t2) <= 8, "{mode:?}: hovered outline {hot:?}, want t2 {:?}", n.t2);
        checks += 2;
        // Focus: the hour segment is selection (ink at 25%) over the fill.
        let focus = win.cx.update(|cx| win.shell.read(cx).popover_time_focus(Field::StartTime, cx)).unwrap();
        win.act(|w, cx| focus.focus(w, cx));
        let hour = win.cx.update_window(win.h, |_, w, _| w.within(id.clone()).find("hour").bounds()).unwrap();
        let seg = mode_of(&win.shot().region(hour, 1.));
        let want = blend(win.theme.ink, rest, 0.25);
        assert!(delta(seg, want) <= 4, "{mode:?}: focused segment {seg:?}, want ink 25% over fill {want:?}");
        let other = win.cx.update_window(win.h, |_, w, _| w.within(id.clone()).find("minute").bounds()).unwrap();
        assert!(delta(mode_of(&win.shot().region(other, 1.)), rest) <= 3, "{mode:?}: the other segment keeps the fill");
        // Like the text inputs, the library marks focus by turning the outline into `ring`; the fill under the segments is kept.
        let ring = edge(&mut win);
        assert!(delta(ring, win.theme.ring) <= 6, "{mode:?}: focus ring {ring:?}, want ring {:?}", win.theme.ring);
        assert!(delta(fill(&mut win, field), rest) <= 3, "{mode:?}: focus keeps the field fill");
        checks += 4;
        // A draft time that cannot be read parks the field at 00:00: it must not look like a valid midnight, so it is invalid (the
        // `danger` role, `ink` in this palette) while the other, readable field keeps the plain outline.
        let mut bad = win.cx.update(|cx| win.shell.read(cx).popover_draft(cx)).unwrap();
        bad.start_time = "nope".into();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_draft(bad, w, cx)));
        win.act(|w, cx| w.pointer_move(point(px(2.), px(2.)), None, cx));
        let (sb, eb) = (win.bounds("pop-start-time-box"), win.bounds("pop-end-time-box"));
        let top = |win: &mut Win, b: gpui_kit::Bounds<gpui_kit::Pixels>| win.shot().mean(b.center().x.into(), f32::from(b.top()) + 0.5, 0.4);
        let (unread, read) = (top(&mut win, sb), top(&mut win, eb));
        assert!(delta(unread, win.theme.ink) <= 8, "{mode:?}: unreadable start time outline {unread:?}, want danger (ink) {:?}", win.theme.ink);
        assert!(delta(read, n.border) <= 8, "{mode:?}: readable end time outline {read:?}, want border {:?}", n.border);
        checks += 2;
        // Disabled: the same field at 50%, so its fill lies halfway to what is behind it.
        win.act(|w, cx| w.click("pop-all-day", cx));
        let dim = fill(&mut win, field);
        assert!(delta(dim, blend(rest, n.surface, 0.5)) <= 4, "{mode:?}: disabled fill {dim:?}, want 50% of {rest:?} over {:?}", n.surface);
        checks += 1;
    }
    println!("time pixels ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
