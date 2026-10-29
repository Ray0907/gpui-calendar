//! The event form, its toast and the notice bar: the text fields are filled from the `field` role (not the library default), cards, All day,
//! footer buttons, Undo and the notice buttons follow the state table, and the modal scrim is the `scrim` role (18% light, 40% dark).
//! Light and dark. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::domain::form::Draft;
    use gpui_calendar::store::WORK;
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::theme::contrast::over;
    use gpui_calendar::theme::contrast::Rgb;
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::TestWindowExt;
    use headless::states::*;
    use headless::*;

    let start = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let event = Event::timed(1, "Piano lesson", WORK, start, start + chrono::Duration::hours(1));
    // The card fades in over 160 ms of wall clock (`popover/view.rs` FADE_MS).
    let open_form = |win: &mut Win| {
        let shell = win.shell.clone();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx)));
        std::thread::sleep(std::time::Duration::from_millis(400));
        win.render();
    };

    // The scrim: what the page showed, darkened by black at the role's alpha.
    for (mode, alpha) in [(Mode::Light, 0.18), (Mode::Dark, 0.40)] {
        let mut win = open(mode, Neutral::Cool, View::Month, vec![event.clone()], (1280., 900.));
        let before = win.shot().at(6., 450.);
        // No anchor, so the card is centered and the page behind it is dimmed.
        let shell = win.shell.clone();
        win.act(|w, cx| shell.update(cx, |s, cx| s.show_draft(Draft::new_at(start, WORK, true), w, cx)));
        std::thread::sleep(std::time::Duration::from_millis(400));
        win.render();
        let got = win.shot().at(6., 450.);
        let want = over(Rgb(0, 0, 0), alpha, before);
        assert!(delta(got, want) <= 3, "{mode:?} scrim over {before:?}: got {got:?}, want black at {alpha} = {want:?}");
    }

    for mode in [Mode::Light, Mode::Dark] {
        date_box(mode);
    }
    let form = event.clone();
    run(
        "form",
        vec![
            Group::new(View::Month, vec![form], |_| {
                vec![
                    // Item 1: the library paints `background` (light) or `input` at 30% (dark); the role is `field`.
                    Spec::fixed("pop-title", |t| t.neutral.field),
                    Spec::fixed("pop-notes-box", |t| t.neutral.field),
                    // Roving radio: focusing the card selects it, so its focus is the selected state (focus_fill.rs), not a ring over a hover.
                    Spec::card("pop-cal-1", |t| t.neutral.surface).nofocus(),
                    Spec::row("pop-all-day", |t| t.neutral.surface),
                    Spec::filled("pop-save"),
                    // The footer bar is `subtle`.
                    Spec::ghost("pop-cancel", |t| t.neutral.surface, |t| t.neutral.subtle),
                ]
            })
            .setup(open_form),
            // All day on: the time fields are disabled, so hover and press change nothing.
            Group::new(View::Month, vec![event.clone()], |_| vec![Spec::disabled("pop-start-time-box", Probe::Top), Spec::disabled("pop-end-time-box", Probe::Top)]).setup(move |win| {
                open_form(win);
                win.act(|w, cx| w.click("pop-all-day", cx));
            }),
            // The toast is an `ink` bar: Undo is a filled ink control like Save (90% on hover, 80% held).
            Group::new(View::Month, vec![event.clone()], |_| vec![Spec::filled("toast-undo").ring_by(|t| t.on_ink)]).setup(move |win| {
                open_form(win);
                win.act(|w, cx| w.click("pop-delete", cx));
            }),
            Group::new(View::Month, vec![], |_| {
                vec![Spec::filled("notice-dismiss").ring_by(|t| t.on_ink)]
            })
            .setup(|win| {
                let shell = win.shell.clone();
                win.act(|_, cx| shell.update(cx, |s, cx| {
                    s.model.notice = "Settings could not be saved.".into();
                    cx.notify();
                }));
            }),
        ],
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
