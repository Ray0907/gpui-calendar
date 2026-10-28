//! Every shipped preset pair meets 4.5 (text) / 3.0 (non-text), using our own contrast math.
use crate::store::settings::{Accent, CalendarColor, Neutral};
use crate::theme::contrast::*;
use crate::theme::palette::*;

const TEXT: f64 = 4.5;
const NON_TEXT: f64 = 3.0;

fn ok(name: String, fg: Rgb, bg: Rgb, need: f64) {
    let v = contrast(fg, bg);
    assert!(v >= need, "{name}: {v:.2} < {need}");
}

#[test]
fn appearance_all_presets_are_listed() {
    assert_eq!(ACCENTS.len(), 8);
    assert_eq!(SLOTS.len(), 8);
    assert_eq!(NEUTRALS.len(), 3);
    // distinct
    for (i, a) in ACCENTS.iter().enumerate() {
        assert!(!ACCENTS[..i].contains(a));
    }
    for (i, a) in SLOTS.iter().enumerate() {
        assert!(!SLOTS[..i].contains(a));
    }
}

#[test]
fn appearance_accent_white_text_and_dark_surface() {
    for n in NEUTRALS {
        let dark_surface = neutral(n, true).surface;
        for a in ACCENTS {
            let c = accent(a).rgb;
            ok(format!("white on {a:?}"), Rgb::WHITE, c, TEXT);
            ok(
                format!("{a:?} on dark surface ({n:?})"),
                c,
                dark_surface,
                NON_TEXT,
            );
        }
    }
}

#[test]
fn appearance_calendar_dot_on_surface_and_weekend() {
    for n in NEUTRALS {
        for dark in [false, true] {
            let t = neutral(n, dark);
            for s in SLOTS {
                let dot = slot(s).scheme(dark).dot;
                ok(
                    format!("{s:?} dot on surface {n:?} dark={dark}"),
                    dot,
                    t.surface,
                    NON_TEXT,
                );
                ok(
                    format!("{s:?} dot on subtle {n:?} dark={dark}"),
                    dot,
                    t.subtle,
                    NON_TEXT,
                );
            }
        }
    }
}

#[test]
fn appearance_text_on_tint() {
    for n in NEUTRALS {
        for dark in [false, true] {
            let surface = neutral(n, dark).surface;
            for s in SLOTS {
                let sc = slot(s).scheme(dark);
                let tint = sc.tint.over(surface);
                ok(
                    format!("{s:?} text on tint {n:?} dark={dark}"),
                    sc.text,
                    tint,
                    TEXT,
                );
            }
        }
    }
}

#[test]
fn appearance_neutral_text_roles_on_every_surface() {
    for n in NEUTRALS {
        for dark in [false, true] {
            let t = neutral(n, dark);
            for (sn, bg) in [
                ("page", t.page),
                ("surface", t.surface),
                ("subtle", t.subtle),
                ("field", t.field),
            ] {
                for (tn, fg) in [("t1", t.t1), ("t2", t.t2), ("t3", t.t3)] {
                    ok(format!("{tn} on {sn} {n:?} dark={dark}"), fg, bg, TEXT);
                }
            }
            ok(
                format!("border on field {n:?} dark={dark}"),
                t.border,
                t.field,
                NON_TEXT,
            );
        }
    }
}

#[test]
fn appearance_banner_text_on_both_gradient_ends() {
    for n in NEUTRALS {
        let b = banner(n);
        for (name, end) in [("from", b.from), ("to", b.to)] {
            ok(format!("label on {name} {n:?}"), b.label, end, TEXT);
            ok(format!("value on {name} {n:?}"), b.value, end, TEXT);
            ok(format!("secondary on {name} {n:?}"), b.secondary, end, TEXT);
        }
    }
}

#[test]
fn appearance_defaults_match_guide() {
    assert_eq!(accent(Accent::Red).rgb, Rgb::hex("#d92d20"));
    assert_eq!(neutral(Neutral::Cool, false).page, Rgb::hex("#f4f5f9"));
    assert_eq!(neutral(Neutral::Cool, true).border, Rgb::hex("#6f7893"));
    let blue = slot(CalendarColor::Blue);
    assert_eq!(blue.light.dot, Rgb::hex("#3b6cf6"));
    assert_eq!(blue.dark.tint.alpha, 0.18);
    assert_eq!(blue.hue, 265);
    assert_eq!(banner(Neutral::Warm).to, Rgb::hex("#3a3126"));
}

#[test]
fn appearance_avatar_gradient_is_the_mocks() {
    assert_eq!(AVATAR, [Rgb::hex("#a78bfa"), Rgb::hex("#60a5fa")]);
}
