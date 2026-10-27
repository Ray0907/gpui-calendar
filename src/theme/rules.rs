//! Collision rules for the Appearance panel and repair of saved values. No GPUI import.
//!
//! One color, one meaning: a swatch whose hue is within 15 degrees of another role is
//! unavailable, and each calendar has its own color.

use super::contrast::hue_distance;
use super::palette::{accent, slot};
use crate::domain::event::CalendarId;
use crate::store::CALENDAR_NAMES as NAMES;
use crate::store::settings::{Accent, Appearance, CalendarColor, CalendarColors};

const HUE_GAP: u16 = 15;

/// Two hues are too close when they are `HUE_GAP` degrees apart or less.
pub fn hue_blocked(a: u16, b: u16) -> bool {
    hue_distance(a, b) <= HUE_GAP
}

/// Calendar colors in `CalendarId` order (`WORK`, `PERSONAL`, `SIDE`).
pub fn calendar_slots(c: &CalendarColors) -> [CalendarColor; 3] {
    [c.work, c.personal, c.side]
}

/// Why `candidate` cannot be the accent, or `None` if it can. The current accent is always available.
pub fn accent_unavailable_reason(a: &Appearance, candidate: Accent) -> Option<String> {
    if candidate == a.accent {
        return None;
    }
    let hue = accent(candidate).hue;
    calendar_slots(&a.calendar_colors)
        .iter()
        .zip(NAMES)
        .find(|(s, _)| hue_blocked(hue, slot(**s).hue))
        .map(|(_, name)| format!("too close to the color of {name}"))
}

/// Why calendar `id` cannot take `candidate`, or `None` if it can. Its own current color is always available.
/// Checked in order: the accent, a color another calendar already uses, a hue too close to another calendar's.
pub fn slot_unavailable_reason(
    a: &Appearance,
    id: CalendarId,
    candidate: CalendarColor,
) -> Option<String> {
    let current = calendar_slots(&a.calendar_colors);
    if current.get(usize::from(id)) == Some(&candidate) {
        return None;
    }
    let hue = slot(candidate).hue;
    if hue_blocked(hue, accent(a.accent).hue) {
        return Some("too close to the accent, which marks today".into());
    }
    let others = || {
        current
            .iter()
            .zip(NAMES)
            .enumerate()
            .filter(|(i, _)| *i != usize::from(id))
    };
    if let Some((_, (_, name))) = others().find(|(_, (s, _))| **s == candidate) {
        return Some(format!("already used by {name}"));
    }
    others()
        .find(|(_, (s, _))| hue_blocked(hue, slot(**s).hue))
        .map(|(_, (_, name))| format!("too close to the color of {name}"))
}

/// `Ok` if the appearance breaks no rule, else the first broken one. Checked in order: a color two calendars
/// share, a calendar too close to the accent, two calendars too close in hue. A shared color is reported even
/// when the accent also blocks it, since that is the more useful message.
pub fn validate(a: &Appearance) -> Result<(), String> {
    let cs = calendar_slots(&a.calendar_colors);
    let pairs = || (0..cs.len()).flat_map(|i| (i + 1..cs.len()).map(move |j| (i, j)));
    if let Some((i, j)) = pairs().find(|&(i, j)| cs[i] == cs[j]) {
        return Err(format!("{} and {} share a color", NAMES[i], NAMES[j]));
    }
    let accent_hue = accent(a.accent).hue;
    if let Some(i) = (0..cs.len()).find(|&i| hue_blocked(slot(cs[i]).hue, accent_hue)) {
        return Err(format!("{} is too close to the accent", NAMES[i]));
    }
    match pairs().find(|&(i, j)| hue_blocked(slot(cs[i]).hue, slot(cs[j]).hue)) {
        Some((i, j)) => Err(format!(
            "{} and {} are too close in hue",
            NAMES[i], NAMES[j]
        )),
        None => Ok(()),
    }
}

/// A valid appearance. Mode and neutral are always valid and kept. If the rest breaks a rule, the fewest
/// of accent, Work, Personal and Side go back to their defaults. Ties between equally small sets are
/// broken the same way every time: a set that resets the accent wins, so the user's calendar picks survive
/// when they can; then the set with the lower mask wins (Work before Personal before Side).
pub fn repair(a: Appearance) -> Appearance {
    let d = Appearance::default();
    let mut masks: Vec<u8> = (0..16).collect(); // bit 0 accent, bits 1 to 3 Work, Personal, Side
    masks.sort_by_key(|m| (m.count_ones(), m & 1 == 0, *m));
    masks
        .into_iter()
        .map(|m| {
            let (mut c, dc) = (a.calendar_colors, d.calendar_colors);
            if m & 2 != 0 {
                c.work = dc.work
            }
            if m & 4 != 0 {
                c.personal = dc.personal
            }
            if m & 8 != 0 {
                c.side = dc.side
            }
            Appearance {
                accent: if m & 1 != 0 { d.accent } else { a.accent },
                calendar_colors: c,
                ..a
            }
        })
        .find(|c| validate(c).is_ok())
        .expect("mask 15 is all defaults, which is valid")
}
