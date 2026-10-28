//! Every library color token comes from a role (design-tokens.md sections 4 and 5). No window.
//!
//! Field names are read from the library, not listed here: `ThemeColor` derives `Serialize`, so its JSON keys are
//! its fields. A field a library upgrade adds appears in `library_names` and fails `table_names_equal_library_fields`.
use std::collections::{BTreeMap, BTreeSet};

use gpui_kit::component::ThemeColor;
use serde_json::Value;

use crate::store::settings::{Accent, Appearance, Mode, Neutral};
use crate::theme::contrast::{Rgb, contrast, over};
use crate::theme::palette::{NEUTRALS, accent};
use crate::theme::resolve::{Theme, resolve};
use crate::theme::token_map::{cal_dot, lay, theme_color_table, theme_colors};

const BOTH: [Mode; 2] = [Mode::Light, Mode::Dark];
const ACCENTS: [Accent; 2] = [Accent::Red, Accent::Blue];

/// Field name to `#rrggbbaa`, as the library serializes it.
fn hexes(c: &ThemeColor) -> BTreeMap<String, String> {
    let Value::Object(m) = serde_json::to_value(c).unwrap() else { panic!("ThemeColor is not an object") };
    m.into_iter().map(|(k, v)| (k, v.as_str().expect("a hex string").to_string())).collect()
}

fn default_of(dark: bool) -> BTreeMap<String, String> {
    hexes(&if dark { *ThemeColor::dark() } else { *ThemeColor::light() })
}

fn library_names() -> BTreeSet<String> {
    default_of(false).into_keys().collect()
}

/// light and dark x 3 neutrals x 2 accents.
fn cases() -> Vec<(String, Neutral, Theme)> {
    let mut v = Vec::new();
    for mode in BOTH {
        for neutral in NEUTRALS {
            for accent in ACCENTS {
                let a = Appearance { mode, accent, neutral, ..Appearance::default() };
                v.push((format!("{mode:?} {neutral:?} {accent:?}"), neutral, resolve(&a, false)));
            }
        }
    }
    v
}

fn rgb_of(hex: &str) -> Rgb {
    Rgb::hex(&hex[..7])
}

fn hex_of(c: Rgb, alpha: f64) -> String {
    format!("#{:06x}{:02x}", c.to_u32(), (alpha * 255.0).round() as u8)
}

/// `#rrggbbaa` strings equal per channel within 1/255, so a library that rounds alpha the other way does not flake.
fn near(got: &str, want: &str) -> bool {
    let ch = |h: &str| (1..9).step_by(2).map(|i| i64::from(u8::from_str_radix(&h[i..i + 2], 16).unwrap())).collect::<Vec<_>>();
    ch(got).iter().zip(ch(want)).all(|(a, b)| (a - b).abs() <= 1)
}

fn solid(c: Rgb) -> String {
    hex_of(c, 1.0)
}

#[test]
fn table_names_equal_library_fields() {
    let lib = library_names();
    for (label, _, t) in cases() {
        let names: Vec<&str> = theme_color_table(&t).iter().map(|(n, _)| *n).collect();
        let set: BTreeSet<String> = names.iter().map(|s| s.to_string()).collect();
        assert_eq!(names.len(), set.len(), "{label}: a field is mapped twice");
        let missing: Vec<_> = lib.difference(&set).collect();
        let extra: Vec<_> = set.difference(&lib).collect();
        assert!(missing.is_empty() && extra.is_empty(), "{label}: library has {} fields; unmapped {missing:?}, unknown {extra:?}", lib.len());
    }
}

/// Fields whose role value is, in that scheme, the very color the library default has: `surface` and `on_ink` are
/// `#ffffff` in every light tint, and the library paints these fields white too. `(field, dark)`.
const SAME_AS_LIBRARY: &[(&str, bool)] = &[("accordion", false), ("list", false), ("popover", false), ("slider_thumb", false), ("switch_thumb", false), ("table", false)];

#[test]
fn no_field_keeps_the_library_default() {
    let mut same = Vec::new();
    for (label, _, t) in cases() {
        let (mapped, lib) = (hexes(&theme_colors(&t)), default_of(t.dark));
        for (name, v) in mapped.iter().filter(|(n, v)| *v == &lib[*n]) {
            if !SAME_AS_LIBRARY.contains(&(name.as_str(), t.dark)) {
                same.push(format!("{label}: {name} {v}"));
            }
        }
    }
    assert!(same.is_empty(), "left at the library default:\n{}", same.join("\n"));
}

/// An allow-list entry must still collide in every case of its scheme, and must hold its role value (`slider_thumb` is
/// `on_ink`, the rest are `surface`): a stale entry or a wrong value fails here.
#[test]
fn same_as_library_entries_are_live_and_hold_their_role() {
    for (name, dark) in SAME_AS_LIBRARY {
        for (label, _, t) in cases().into_iter().filter(|(_, _, t)| t.dark == *dark) {
            let (mapped, lib) = (hexes(&theme_colors(&t)), default_of(t.dark));
            assert_eq!(mapped[*name], lib[*name], "{label}: `{name}` no longer collides with the library, drop it from SAME_AS_LIBRARY");
            let role = if *name == "slider_thumb" { t.on_ink } else { t.neutral.surface };
            assert_eq!(mapped[*name], solid(role), "{label}: `{name}` is not its role value");
        }
    }
}

/// design-tokens 2 `disabled` (t3 label, 50% opacity) is applied by the view, not mapped: `ThemeColor` has no disabled field.
#[test]
fn no_field_is_named_disabled() {
    assert!(library_names().iter().all(|n| !n.contains("disabled")), "the library gained a disabled field: map it and update design-tokens 2");
}

/// Hover and active fields by family: `(hover, active, rest)`. Wash fields sit on their backdrop (`rest` is the backdrop
/// role); filled-ink fields rest on their own fill; `scrollbar_thumb_hover` is an alpha step of its thumb.
/// `list_active`, `table_active` and `tab_active` are selected fills, `link_hover`/`link_active` are text: not states of a press.
const EXEMPT: [&str; 5] = ["list_active", "table_active", "tab_active", "link_hover", "link_active"];

enum Family {
    Wash(&'static str, Option<&'static str>, fn(&Theme) -> Rgb),
    Ink(&'static str, &'static str, &'static str),
}

fn families() -> Vec<Family> {
    use Family::*;
    vec![
        Wash("secondary_hover", Some("secondary_active"), |t| t.neutral.surface),
        Wash("button_hover", Some("button_active"), |t| t.neutral.page),
        Wash("button_secondary_hover", Some("button_secondary_active"), |t| t.neutral.field),
        Wash("list_hover", None, |t| t.neutral.surface),
        Wash("table_hover", None, |t| t.neutral.surface),
        Wash("sidebar_accent", None, |t| t.neutral.surface),
        Ink("primary", "primary_hover", "primary_active"),
        Ink("danger", "danger_hover", "danger_active"),
        Ink("info", "info_hover", "info_active"),
        Ink("success", "success_hover", "success_active"),
        Ink("warning", "warning_hover", "warning_active"),
        Ink("button_primary", "button_primary_hover", "button_primary_active"),
        Ink("button_danger", "button_danger_hover", "button_danger_active"),
        Ink("button_info", "button_info_hover", "button_info_active"),
        Ink("button_success", "button_success_hover", "button_success_active"),
        Ink("button_warning", "button_warning_hover", "button_warning_active"),
    ]
}

#[test]
fn every_hover_and_active_field_belongs_to_a_family() {
    let mut named: BTreeSet<&str> = EXEMPT.into_iter().chain(["scrollbar_thumb_hover"]).collect();
    for f in families() {
        match f {
            Family::Wash(h, a, _) => named.extend([h].into_iter().chain(a)),
            Family::Ink(_, h, a) => named.extend([h, a]),
        }
    }
    let loose: Vec<_> = library_names().into_iter().filter(|n| (n.ends_with("_hover") || n.ends_with("_active") || n == "sidebar_accent") && !named.contains(n.as_str())).collect();
    assert!(loose.is_empty(), "hover or active fields with no family (design-states 3): {loose:?}");
}

/// design-states 3: a wash hover differs from its backdrop and stays quiet (contrast 1.08..1.25; luminance <= 0.10 in dark),
/// a press is a step further (1.18..1.40, active != hover). A filled-ink hover is ink at 90% over page and active at 80%: exact.
#[test]
fn hover_and_active_states_follow_the_family_rules() {
    for (label, _, t) in cases() {
        let m = hexes(&theme_colors(&t));
        for f in families() {
            match f {
                Family::Wash(hover, active, backdrop) => {
                    let backdrop = backdrop(&t);
                    let h = rgb_of(&m[hover]);
                    assert_ne!(h, backdrop, "{label}: {hover} equals its backdrop");
                    let c = contrast(h, backdrop);
                    assert!((1.08..=1.25).contains(&c), "{label}: {hover} contrast {c:.3} against its backdrop");
                    if t.dark {
                        assert!(h.luminance() <= 0.10, "{label}: dark {hover} {h:?} is near-white (luminance {:.3})", h.luminance());
                    }
                    if let Some(active) = active {
                        let a = rgb_of(&m[active]);
                        assert_ne!(a, h, "{label}: {active} equals {hover}");
                        let c = contrast(a, backdrop);
                        assert!((1.18..=1.40).contains(&c), "{label}: {active} contrast {c:.3} against its backdrop");
                        if t.dark {
                            assert!(a.luminance() <= 0.10, "{label}: dark {active} {a:?} is near-white");
                        }
                    }
                }
                Family::Ink(rest, hover, active) => {
                    let (r, h, a) = (rgb_of(&m[rest]), rgb_of(&m[hover]), rgb_of(&m[active]));
                    assert_eq!(r, t.ink, "{label}: {rest} is not ink");
                    assert_eq!(h, over(t.ink, 0.9, t.neutral.page), "{label}: {hover} is not ink at 90% over page");
                    assert_eq!(a, over(t.ink, 0.8, t.neutral.page), "{label}: {active} is not ink at 80% over page");
                    assert!(h != r && a != h, "{label}: {rest} hover or active does not dim");
                }
            }
        }
        let (rest, hover) = (&m["scrollbar_thumb"], &m["scrollbar_thumb_hover"]);
        assert_ne!(rest, hover, "{label}: scrollbar thumb hover equals rest");
    }
}

#[test]
fn ink_is_never_a_hover_or_active_fill() {
    for (label, _, t) in cases() {
        for (name, v) in hexes(&theme_colors(&t)) {
            let state = name.ends_with("_hover") || name.ends_with("_active") || name == "sidebar_accent";
            // Link hover and active are text colors, not fills (design-tokens 4: links are text).
            if !state || name.starts_with("link") {
                continue;
            }
            let c = rgb_of(&v);
            assert!(c != t.ink && c != t.neutral.t1 && c != Rgb::WHITE, "{label}: `{name}` is ink, t1 or white");
        }
    }
}

/// One row per group of design-tokens section 4: the field must be the role, computed here without the adapter.
#[test]
fn roles_resolve() {
    for (label, _, t) in cases() {
        let (n, m) = (&t.neutral, hexes(&theme_colors(&t)));
        let hv = |b: Rgb| over(n.t1, 0.06, b);
        let pr = |b: Rgb| over(n.t1, 0.10, b);
        let ink = |a: f64| over(t.ink, a, n.page);
        let dots: Vec<Rgb> = (0..3).map(|i| t.cal(i).unwrap().dot).collect();
        let rows: Vec<(&str, String)> = vec![
            ("secondary_hover", solid(hv(n.surface))),
            ("secondary_active", solid(pr(n.surface))),
            ("button_hover", solid(hv(n.page))),
            ("button_active", solid(pr(n.page))),
            ("button_foreground", solid(n.t1)),
            ("button_secondary", solid(n.field)),
            ("button_secondary_hover", solid(hv(n.field))),
            ("button_primary", solid(t.ink)),
            ("button_primary_hover", solid(ink(0.9))),
            ("button_primary_active", solid(ink(0.8))),
            ("button_primary_foreground", solid(t.on_ink)),
            ("button_danger_hover", solid(ink(0.9))),
            ("info", solid(t.ink)),
            ("success_active", solid(ink(0.8))),
            ("warning_foreground", solid(t.on_ink)),
            ("danger", solid(t.ink)),
            ("danger_foreground", solid(t.on_ink)),
            ("primary_hover", solid(ink(0.9))),
            ("primary_active", solid(ink(0.8))),
            ("caret", solid(t.ink)),
            ("selection", hex_of(t.ink, 0.25)),
            ("scrollbar", solid(n.surface)),
            ("scrollbar_thumb", hex_of(n.t3, 0.45)),
            ("scrollbar_thumb_hover", hex_of(n.t3, 0.70)),
            ("list", solid(n.surface)),
            ("list_hover", solid(hv(n.surface))),
            ("list_active", solid(n.subtle)),
            ("list_active_border", solid(t.ink)),
            ("table_hover", solid(hv(n.surface))),
            ("table_row_border", solid(n.line)),
            ("table_head_foreground", solid(n.t3)),
            ("tab_active", solid(n.page)),
            ("tab_bar_segmented", solid(n.field)),
            ("sidebar_accent", solid(hv(n.surface))),
            ("sidebar_primary", solid(t.ink)),
            ("sidebar_primary_foreground", solid(t.on_ink)),
            ("title_bar_border", solid(n.line)),
            ("window_border", solid(n.line)),
            ("link", solid(n.t2)),
            ("link_hover", solid(n.t1)),
            ("switch_thumb", solid(n.surface)),
            ("slider_thumb", solid(t.on_ink)),
            ("drag_border", hex_of(t.ink, 0.65)),
            ("drop_target", hex_of(t.ink, 0.20)),
            ("progress_bar", solid(t.ink)),
            ("skeleton", solid(n.field)),
            ("group_box_foreground", solid(n.t2)),
            ("description_list_label", solid(n.subtle)),
            ("chart_1", solid(dots[0])),
            ("chart_2", solid(dots[1])),
            ("chart_3", solid(dots[2])),
            ("chart_grid", solid(n.line)),
            ("chart_bullish", solid(n.t2)),
        ];
        let mut rows = rows;
        for (name, light, preset) in [("red", "red_light", Accent::Red), ("green", "green_light", Accent::Green), ("blue", "blue_light", Accent::Blue), ("yellow", "yellow_light", Accent::Orange), ("magenta", "magenta_light", Accent::Pink), ("cyan", "cyan_light", Accent::Teal)] {
            rows.push((name, solid(accent(preset).rgb)));
            rows.push((light, solid(over(accent(preset).rgb, 0.2, n.surface))));
        }
        for (name, want) in rows {
            assert!(near(&m[name], &want), "{label}: `{name}` is {} want {want}", m[name]);
        }
        // The scrim is black; its alpha is the role (18% light, 40% dark).
        let scrim = &m["overlay"];
        assert_eq!(&scrim[..7], "#000000", "{label}: overlay is a black scrim");
        assert!(near(scrim, &hex_of(Rgb(0, 0, 0), if t.dark { 0.40 } else { 0.18 })), "{label}: overlay alpha {scrim}");
    }
}

#[test]
#[should_panic(expected = "bogus_field")]
fn an_unknown_table_field_panics_with_its_name() {
    let t = &cases()[0].2;
    lay(*ThemeColor::light(), vec![("bogus_field", crate::theme::gpui_adapter::hsla(t.ink))]);
}

#[test]
#[should_panic(expected = "calendar 3")]
fn a_missing_calendar_slot_panics() {
    cal_dot(&cases()[0].2, 3);
}
