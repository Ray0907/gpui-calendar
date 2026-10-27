//! `Appearance` to concrete colors. No GPUI import.

use super::contrast::Rgb;
use super::palette::{Banner, NeutralSet, accent, banner, neutral, slot};
use super::rules::{calendar_slots, repair};
use crate::domain::event::CalendarId;
use crate::store::settings::{Appearance, Mode};

/// One calendar's colors in the resolved theme. `tint` is already composited over the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarTheme {
    pub dot: Rgb,
    pub tint: Rgb,
    pub text: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub dark: bool,
    pub neutral: NeutralSet,
    /// Today's date marker; `accent_on` is the text on it.
    pub accent: Rgb,
    pub accent_on: Rgb,
    /// Primary button, selected state, app mark; `ring` is the focus ring (same as `ink`).
    pub ink: Rgb,
    pub on_ink: Rgb,
    pub ring: Rgb,
    pub banner: Banner,
    cals: [CalendarTheme; 3],
}

impl Theme {
    /// Colors of calendar `id` (`WORK`, `PERSONAL`, `SIDE`); `None` for an id no calendar has.
    pub fn cal(&self, id: CalendarId) -> Option<CalendarTheme> {
        self.cals.get(usize::from(id)).copied()
    }
}

/// Resolve `appearance` (repaired first, so a rule-breaking value never reaches the UI).
/// `Mode::System` follows `system_dark`. Use `resolve_repaired` to learn whether the value was changed.
pub fn resolve(appearance: &Appearance, system_dark: bool) -> Theme {
    resolve_repaired(appearance, system_dark).0
}

/// Like `resolve`, plus the repaired `Appearance` when the input broke a rule (`None` if it was valid).
/// The theme is built from the repaired value, so a settings panel showing the saved value would disagree
/// with what renders; show or persist the returned value instead.
pub fn resolve_repaired(appearance: &Appearance, system_dark: bool) -> (Theme, Option<Appearance>) {
    let a = repair(*appearance);
    let repaired = (a != *appearance).then_some(a);
    let dark = match a.mode {
        Mode::Light => false,
        Mode::Dark => true,
        Mode::System => system_dark,
    };
    let n = *neutral(a.neutral, dark);
    let cals = calendar_slots(&a.calendar_colors).map(|c| {
        let s = slot(c).scheme(dark);
        CalendarTheme {
            dot: s.dot,
            tint: s.tint.over(n.surface),
            text: s.text,
        }
    });
    let ink = n.t1;
    let theme = Theme {
        dark,
        neutral: n,
        accent: accent(a.accent).rgb,
        accent_on: Rgb::WHITE,
        ink,
        on_ink: if dark { n.page } else { n.surface },
        ring: ink,
        banner: *banner(a.neutral),
        cals,
    };
    (theme, repaired)
}
