use crate::store::settings::{
    Accent, Appearance, CalendarColor as C, CalendarColors, Mode, Neutral,
};
use crate::store::{PERSONAL, SIDE, WORK};
use crate::theme::rules::*;

fn app(accent: Accent, w: C, p: C, s: C) -> Appearance {
    Appearance {
        mode: Mode::System,
        accent,
        neutral: Neutral::Cool,
        calendar_colors: CalendarColors {
            work: w,
            personal: p,
            side: s,
        },
    }
}

#[test]
fn appearance_defaults_are_valid() {
    assert!(validate(&Appearance::default()).is_ok());
}

#[test]
fn appearance_selected_swatch_is_never_unavailable() {
    let a = Appearance::default();
    assert_eq!(accent_unavailable_reason(&a, a.accent), None);
    assert_eq!(slot_unavailable_reason(&a, WORK, C::Blue), None);
}

#[test]
fn appearance_accent_blocked_by_close_calendar_hue() {
    let a = Appearance::default(); // Blue/Green/Amber calendars
    // Blue 258 vs Blue slot 265 (7), Indigo 278 vs 265 (13), Green 150 vs 152 (2)
    let r = accent_unavailable_reason(&a, Accent::Blue).unwrap();
    assert_eq!(r, "too close to the color of Work");
    assert!(accent_unavailable_reason(&a, Accent::Indigo).is_some());
    assert_eq!(
        accent_unavailable_reason(&a, Accent::Green).unwrap(),
        "too close to the color of Personal"
    );
    // Orange 48 vs Amber 80 is 32 apart: allowed. Teal, Violet, Pink, Orange are free.
    for ok in [Accent::Orange, Accent::Teal, Accent::Violet, Accent::Pink] {
        assert_eq!(accent_unavailable_reason(&a, ok), None, "{ok:?}");
    }
}

#[test]
fn appearance_teal_accent_blocks_only_the_teal_slot() {
    // Teal accent hue 190: Teal slot 195 is 5 away; Red (29) and Blue (265) are far.
    let a = app(Accent::Teal, C::Blue, C::Green, C::Amber);
    assert_eq!(
        slot_unavailable_reason(&a, SIDE, C::Teal).unwrap(),
        "too close to the accent, which marks today"
    );
    assert_eq!(slot_unavailable_reason(&a, SIDE, C::Red), None);
    assert_eq!(slot_unavailable_reason(&a, SIDE, C::Pink), None);
}

#[test]
fn appearance_default_red_accent_blocks_red_slot_and_boundary_is_inclusive() {
    let a = Appearance::default();
    assert!(slot_unavailable_reason(&a, SIDE, C::Red).is_some());
    // exactly 15 apart is blocked, 16 is not.
    assert!(hue_blocked(100, 115));
    assert!(!hue_blocked(100, 116));
    assert!(hue_blocked(355, 10));
}

#[test]
fn appearance_indigo_accent_blocks_blue_slot_for_other_calendars() {
    let a = app(Accent::Indigo, C::Amber, C::Green, C::Pink);
    assert!(slot_unavailable_reason(&a, PERSONAL, C::Blue).is_some());
}

#[test]
fn appearance_slot_already_used_by_other_calendar() {
    let a = Appearance::default();
    assert_eq!(
        slot_unavailable_reason(&a, PERSONAL, C::Blue).unwrap(),
        "already used by Work"
    );
    assert_eq!(
        slot_unavailable_reason(&a, WORK, C::Amber).unwrap(),
        "already used by Side project"
    );
    assert_eq!(slot_unavailable_reason(&a, WORK, C::Teal), None);
}

#[test]
fn appearance_validate_rejects_duplicates_and_accent_clash() {
    assert!(validate(&app(Accent::Red, C::Blue, C::Blue, C::Amber)).is_err());
    assert!(validate(&app(Accent::Red, C::Red, C::Green, C::Amber)).is_err());
    assert!(validate(&app(Accent::Teal, C::Blue, C::Green, C::Teal)).is_err());
    assert!(validate(&app(Accent::Teal, C::Blue, C::Green, C::Amber)).is_ok());
}

#[test]
fn appearance_repair_keeps_valid_and_is_deterministic() {
    let ok = app(Accent::Teal, C::Pink, C::Violet, C::Orange);
    assert_eq!(repair(ok), ok);

    // calendars clash with each other: accent kept, calendars reset.
    let bad = Appearance {
        mode: Mode::Dark,
        neutral: Neutral::Warm,
        ..app(Accent::Teal, C::Blue, C::Blue, C::Amber)
    };
    let fixed = repair(bad);
    assert_eq!(
        fixed,
        Appearance {
            mode: Mode::Dark,
            neutral: Neutral::Warm,
            ..Appearance {
                accent: Accent::Teal,
                ..Appearance::default()
            }
        }
    );
    assert_eq!(repair(bad), fixed);
    assert!(validate(&fixed).is_ok());

    // accent clashes with the (valid) default calendars and calendars are custom: accent falls back.
    let bad = app(Accent::Teal, C::Red, C::Teal, C::Pink);
    let fixed = repair(bad);
    assert!(validate(&fixed).is_ok());

    // both broken: everything default (mode and neutral untouched).
    let bad = Appearance {
        mode: Mode::Light,
        neutral: Neutral::Neutral,
        ..app(Accent::Blue, C::Blue, C::Blue, C::Blue)
    };
    let fixed = repair(bad);
    assert_eq!(
        fixed,
        Appearance {
            mode: Mode::Light,
            neutral: Neutral::Neutral,
            ..Appearance::default()
        }
    );
}

#[test]
fn appearance_every_repaired_combination_is_valid() {
    use crate::theme::palette::{ACCENTS, SLOTS};
    for &ac in &ACCENTS {
        for &w in &SLOTS {
            for &p in &SLOTS {
                for &s in &SLOTS {
                    assert!(validate(&repair(app(ac, w, p, s))).is_ok());
                }
            }
        }
    }
}

#[test]
fn appearance_repair_accent_clash_keeps_user_calendar_picks() {
    // Pink, Violet and Orange are valid together; only the Orange accent (48) clashes with the Orange slot (55).
    let keep = app(Accent::Teal, C::Pink, C::Violet, C::Orange);
    let bad = app(Accent::Orange, C::Pink, C::Violet, C::Orange);
    assert!(validate(&bad).is_err());
    let fixed = repair(bad);
    assert_eq!(
        calendar_slots(&fixed.calendar_colors),
        [C::Pink, C::Violet, C::Orange]
    );
    assert_eq!(fixed.accent, Appearance::default().accent);
    assert!(validate(&fixed).is_ok());
    assert_eq!(repair(keep), keep);
}

#[test]
fn appearance_repair_changes_the_fewest_fields() {
    use crate::theme::palette::{ACCENTS, SLOTS};
    let d = Appearance::default();
    let changed = |a: Appearance, b: Appearance| {
        usize::from(a.accent != b.accent)
            + calendar_slots(&a.calendar_colors)
                .iter()
                .zip(calendar_slots(&b.calendar_colors))
                .filter(|(x, y)| **x != *y)
                .count()
    };
    // bit 0 accent, bits 1 to 3 Work, Personal, Side: reset those fields to their defaults.
    let reset = |a: Appearance, m: u8| {
        let mut c = a.calendar_colors;
        if m & 2 != 0 {
            c.work = d.calendar_colors.work
        }
        if m & 4 != 0 {
            c.personal = d.calendar_colors.personal
        }
        if m & 8 != 0 {
            c.side = d.calendar_colors.side
        }
        Appearance {
            accent: if m & 1 != 0 { d.accent } else { a.accent },
            calendar_colors: c,
            ..a
        }
    };
    let mut by_size = [0usize; 5];
    for &ac in &ACCENTS {
        for &w in &SLOTS {
            for &p in &SLOTS {
                for &s in &SLOTS {
                    let bad = app(ac, w, p, s);
                    let fixed = repair(bad);
                    assert!(validate(&fixed).is_ok());
                    let best = (0..16u8)
                        .map(|m| reset(bad, m))
                        .filter(|c| validate(c).is_ok())
                        .map(|c| changed(bad, c))
                        .min()
                        .unwrap();
                    assert_eq!(changed(bad, fixed), best, "{bad:?} -> {fixed:?}");
                    by_size[best] += 1;
                }
            }
        }
    }
    // The sweep must reach conflicts that need 2 and 3 fields, or the minimality check proves nothing there.
    assert!(by_size[2] > 0 && by_size[3] > 0, "{by_size:?}");
}

#[test]
fn appearance_repair_tie_resets_accent_first_then_work_before_personal_before_side() {
    // Accent Orange (48) clashes with the Orange slot (55) on Side; resetting the accent or Side both fix it. Accent goes.
    let fixed = repair(app(Accent::Orange, C::Pink, C::Violet, C::Orange));
    assert_eq!(
        (fixed.accent, calendar_slots(&fixed.calendar_colors)),
        (
            Appearance::default().accent,
            [C::Pink, C::Violet, C::Orange]
        )
    );
    // Work and Personal both Pink: resetting either fixes it. Work goes, Personal keeps its pick.
    let fixed = repair(app(Accent::Teal, C::Pink, C::Pink, C::Amber));
    assert_eq!(
        calendar_slots(&fixed.calendar_colors),
        [
            Appearance::default().calendar_colors.work,
            C::Pink,
            C::Amber
        ]
    );
    // Personal and Side both Pink: only one of them need change, Personal before Side.
    let fixed = repair(app(Accent::Teal, C::Blue, C::Pink, C::Pink));
    assert_eq!(
        calendar_slots(&fixed.calendar_colors),
        [
            C::Blue,
            Appearance::default().calendar_colors.personal,
            C::Pink
        ]
    );
}

#[test]
fn appearance_validate_reports_shared_color_even_when_accent_blocks_it() {
    // Teal accent also blocks the Teal slot, but the duplicate is the more useful message.
    let r = validate(&app(Accent::Teal, C::Teal, C::Green, C::Teal)).unwrap_err();
    assert_eq!(r, "Work and Side project share a color");
}
