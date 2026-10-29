//! Hover on every popover field, light and dark (state table, Field): the outline turns `t2`; a focused field keeps the `ring` outline
//! even with the pointer over it. Title, Notes, both day pickers and both time fields behave alike. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::domain::form::Field;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestWindowExt};
    use gpui_kit::{Bounds, Pixels, point, px};
    use headless::*;

    let start = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let event = Event::timed(1, "Piano lesson", WORK, start, start + chrono::Duration::hours(1));
    let mut checks = 0;
    for mode in [Mode::Light, Mode::Dark] {
        let mut win = open(mode, Neutral::Cool, View::Month, vec![event.clone()], (1280., 900.));
        let shell = win.shell.clone();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx)));
        // The card fades in over 160 ms of wall clock (`popover/view.rs` FADE_MS).
        std::thread::sleep(std::time::Duration::from_millis(400));
        win.render();
        let (t2, ring) = (win.theme.neutral.t2, win.theme.ring);
        assert!(delta(t2, ring) > 16, "{mode:?}: t2 {t2:?} and ring {ring:?} must differ for this test to tell them apart");
        // (name, the field's own bounds, how to focus it)
        type Kind = (&'static str, Box<dyn Fn(&mut Win) -> Bounds<Pixels>>, Box<dyn Fn(&mut Win)>);
        let kinds: [Kind; 6] = [
            ("Title", Box::new(|w| w.bounds("pop-title")), Box::new(|w| w.act(|win, cx| win.click("pop-title", cx)))),
            ("Notes", Box::new(|w| w.bounds("pop-notes-box")), Box::new(|w| w.act(|win, cx| win.click("pop-notes-box", cx)))),
            ("Start date", Box::new(|w| w.bounds("pop-start-date-box")), Box::new(|w| focus_date(w, Field::StartDate))),
            ("End date", Box::new(|w| w.bounds("pop-end-date-box")), Box::new(|w| focus_date(w, Field::EndDate))),
            ("Start time", Box::new(|w| w.bounds("pop-start-time-box")), Box::new(|w| focus_time(w, Field::StartTime))),
            ("End time", Box::new(|w| w.bounds("pop-end-time-box")), Box::new(|w| focus_time(w, Field::EndTime))),
        ];
        fn focus_date(w: &mut Win, f: Field) {
            let focus = w.cx.update(|cx| w.shell.read(cx).popover_date_focus(f, cx)).unwrap();
            w.act(|win, cx| focus.focus(win, cx));
        }
        fn focus_time(w: &mut Win, f: Field) {
            let focus = w.cx.update(|cx| w.shell.read(cx).popover_time_focus(f, cx)).unwrap();
            w.act(|win, cx| focus.focus(win, cx));
        }
        for (name, bounds, focus) in &kinds {
            // Nothing in a field focused (Title has it on open), pointer away: the outline at rest.
            let away = point(px(2.), px(2.));
            let trap = win.cx.update(|cx| win.shell.read(cx).popover_focus()).unwrap();
            win.act(|w, cx| trap.focus(w, cx));
            win.act(|w, cx| w.pointer_move(away, None, cx));
            let b = bounds(&mut win);
            let edge = |win: &mut Win| win.shot().mean(b.center().x.into(), f32::from(b.top()) + 0.5, 0.4);
            let rest = edge(&mut win);
            assert!(delta(rest, t2) > 8, "{mode:?} {name}: the resting outline {rest:?} already reads as t2 {t2:?}");
            win.act(|w, cx| w.pointer_move(b.center(), None, cx));
            let hot = edge(&mut win);
            assert!(delta(hot, t2) <= 8, "{mode:?} {name}: hovered outline {hot:?}, want t2 {t2:?}");
            // Focused and hovered: the ring wins. The pointer leaves and comes back, as a user's would.
            focus(&mut win);
            win.act(|w, cx| w.pointer_move(away, None, cx));
            win.act(|w, cx| w.pointer_move(b.center(), None, cx));
            let b = bounds(&mut win);
            let lit = win.shot().mean(b.center().x.into(), f32::from(b.top()) + 0.5, 0.4);
            // The date box shows its ring for keyboard focus only (design-states 3); the pointer moves above end that modality.
            let want = if name.ends_with("date") { win.theme.neutral.border } else { ring };
            assert!(delta(lit, want) <= 6, "{mode:?} {name}: focused and hovered outline {lit:?}, want {want:?} (not t2 {t2:?})");
            // Back to rest for the next field: pointer away, focus on the card.
            checks += 3;
        }
    }
    println!("field hover ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
