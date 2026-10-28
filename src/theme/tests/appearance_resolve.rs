use crate::store::settings::{Accent, Appearance, CalendarColor as C, Mode, Neutral};
use crate::store::{PERSONAL, SIDE, WORK};
use crate::theme::contrast::{Rgb, contrast};
use crate::theme::palette::{ACCENTS, NEUTRALS, accent, neutral, slot};
use crate::theme::resolve::{resolve, resolve_repaired};
use crate::theme::rules::repair;

fn with_mode(mode: Mode) -> Appearance {
    Appearance {
        mode,
        ..Appearance::default()
    }
}

#[test]
fn appearance_system_mode_follows_flag() {
    assert!(!resolve(&with_mode(Mode::System), false).dark);
    assert!(resolve(&with_mode(Mode::System), true).dark);
}

#[test]
fn appearance_explicit_mode_ignores_system_flag() {
    for sys in [false, true] {
        assert!(!resolve(&with_mode(Mode::Light), sys).dark);
        assert!(resolve(&with_mode(Mode::Dark), sys).dark);
    }
}

#[test]
fn appearance_resolve_picks_the_chosen_presets() {
    let a = Appearance {
        accent: Accent::Teal,
        neutral: Neutral::Warm,
        mode: Mode::Dark,
        ..Appearance::default()
    };
    let t = resolve(&a, false);
    assert_eq!(t.accent, accent(Accent::Teal).rgb);
    assert_eq!(t.neutral, *neutral(Neutral::Warm, true));
    assert_eq!(t.accent_on, Rgb::WHITE);
    assert_eq!(t.cal(WORK).unwrap().dot, slot(C::Blue).dark.dot);
    assert_eq!(t.cal(PERSONAL).unwrap().dot, slot(C::Green).dark.dot);
    assert_eq!(t.cal(SIDE).unwrap().dot, slot(C::Amber).dark.dot);
    // ink pair: dark text-1 on page
    assert_eq!(t.ink, t.neutral.t1);
    assert_eq!(t.on_ink, t.neutral.page);
    assert_eq!(t.ring, t.ink);
}

#[test]
fn appearance_resolve_light_ink_is_dark_on_white() {
    let t = resolve(&with_mode(Mode::Light), true);
    assert_eq!(t.ink, Rgb::hex("#161b2b"));
    assert_eq!(t.on_ink, Rgb::WHITE);
}

#[test]
fn appearance_resolve_repairs_invalid_input() {
    let bad = Appearance {
        accent: Accent::Blue,
        ..Appearance::default()
    }; // Blue accent vs Work Blue
    let t = resolve(&bad, false);
    // calendars fall back to the defaults that the repair keeps valid
    assert_eq!(t.cal(WORK).unwrap().dot, slot(C::Blue).light.dot);
    assert_ne!(t.accent, accent(Accent::Blue).rgb);
}

#[test]
fn appearance_resolve_repaired_surfaces_the_repair() {
    let bad = Appearance {
        accent: Accent::Blue,
        ..Appearance::default()
    };
    let (t, repaired) = resolve_repaired(&bad, false);
    assert_eq!(repaired, Some(repair(bad)));
    assert_eq!(t, resolve(&bad, false));
    let good = Appearance::default();
    assert_eq!(resolve_repaired(&good, false).1, None);
}

#[test]
fn appearance_resolve_is_deterministic() {
    let a = Appearance::default();
    assert_eq!(resolve(&a, true), resolve(&a, true));
}

#[test]
fn appearance_tint_is_composited_over_surface() {
    let t = resolve(&with_mode(Mode::Dark), false);
    let c = t.cal(WORK).unwrap();
    assert_eq!(c.tint, slot(C::Blue).dark.tint.over(t.neutral.surface));
}

#[test]
fn appearance_cal_out_of_range_is_none_not_clamped() {
    let t = resolve(&Appearance::default(), false);
    for id in [WORK, PERSONAL, SIDE] {
        assert!(t.cal(id).is_some());
    }
    assert_eq!(t.cal(3), None);
    assert_eq!(t.cal(7), None);
    assert_eq!(t.cal(u8::MAX), None);
}

#[test]
fn appearance_resolved_themes_keep_accent_and_ink_readable() {
    for mode in [Mode::Light, Mode::Dark, Mode::System] {
        for sys in [false, true] {
            for neutral in NEUTRALS {
                for accent in ACCENTS {
                    let t = resolve(
                        &Appearance {
                            mode,
                            accent,
                            neutral,
                            ..Appearance::default()
                        },
                        sys,
                    );
                    let ctx = format!("{mode:?} sys={sys} {neutral:?} {accent:?}");
                    let (a, i) = (contrast(t.accent_on, t.accent), contrast(t.on_ink, t.ink));
                    assert!(a >= 4.5, "accent_on/accent {a:.2} {ctx}");
                    assert!(i >= 4.5, "on_ink/ink {i:.2} {ctx}");
                    for id in [WORK, PERSONAL, SIDE] {
                        let c = t.cal(id).unwrap();
                        let v = contrast(c.text, c.tint);
                        assert!(v >= 4.5, "text/tint {v:.2} cal {id} {ctx}");
                    }
                }
            }
        }
    }
}

#[test]
fn appearance_calendar_check_reads_on_every_dot_in_both_schemes() {
    use crate::theme::palette::SLOTS;
    use crate::theme::resolve::check_color;
    for dark in [false, true] {
        for nt in NEUTRALS {
            let n = neutral(nt, dark);
            for c in SLOTS {
                let dot = slot(c).scheme(dark).dot;
                let ratio = contrast(check_color(dot, n), dot);
                assert!(ratio >= 3.0, "{c:?} dark={dark} {nt:?}: check {ratio:.2}:1");
            }
        }
    }
    // The theme hands the views that same color for each calendar.
    for sys in [false, true] {
        let t = resolve(&with_mode(Mode::System), sys);
        for id in [WORK, PERSONAL, SIDE] {
            let k = t.cal(id).unwrap();
            assert_eq!(k.check, check_color(k.dot, &t.neutral));
        }
    }
}
