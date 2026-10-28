//! The Appearance screen: swatches, tint buttons, mode pills and Reset follow the state table, light and dark. An unavailable swatch shows no
//! hover. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::Neutral;
    use gpui_calendar::theme::palette::{ACCENTS, SLOTS};
    use gpui_calendar::ui::model::View;
    use headless::states::*;

    let open_screen = |win: &mut headless::Win| {
        let shell = win.shell.clone();
        win.act(|_, cx| shell.update(cx, |s, cx| {
            s.model.toggle_appearance();
            cx.notify();
        }));
    };
    run(
        "appearance",
        vec![Group::new(View::Month, vec![], |win| {
            let (accent, color) = win.cx.update(|cx| {
                let m = &win.shell.read(cx).model;
                let cur = m.store.settings.appearance;
                let a = ACCENTS.iter().find(|&&a| a != cur.accent && m.accent_reason(a).is_none()).expect("a free accent");
                let k = SLOTS.iter().find(|&&k| m.calendar_color(0) != Some(k) && m.calendar_color_reason(0, k).is_none()).expect("a free calendar color");
                (format!("accent-{a:?}"), format!("calendar-color-0-{k:?}"))
            });
            let off = win.cx.update(|cx| {
                let m = &win.shell.read(cx).model;
                let cur = m.store.settings.appearance;
                ACCENTS.iter().find(|&&a| a != cur.accent && m.accent_reason(a).is_some()).map(|a| format!("accent-{a:?}")).or_else(|| SLOTS.iter().find(|&&k| m.calendar_color_reason(0, k).is_some()).map(|k| format!("calendar-color-0-{k:?}")))
            });
            let mode_on = if win.theme.dark { "mode-dark" } else { "mode-light" };
            let mode_off = if win.theme.dark { "mode-light" } else { "mode-dark" };
            vec![
                // The target is a 36px circle around a 24px dot: sample the ring beside the dot.
                Spec::ghost(&accent, |t| t.neutral.surface, |t| t.neutral.surface).at(Probe::Left(3.)),
                Spec::ghost(&color, |t| t.neutral.surface, |t| t.neutral.surface).at(Probe::Left(3.)),
                Spec::card(&format!("neutral-{:?}", Neutral::Warm), |t| t.neutral.surface),
                Spec::filled(mode_on),
                Spec::ghost(mode_off, |t| t.neutral.surface, |t| t.neutral.surface),
                // Unavailable: no hover, no press (state table).
                Spec::disabled(&off.expect("an unavailable swatch"), Probe::Left(3.)),
                Spec::ghost("reset-appearance", |t| t.neutral.surface, |t| t.neutral.surface),
            ]
        })
        .setup(open_screen)],
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
