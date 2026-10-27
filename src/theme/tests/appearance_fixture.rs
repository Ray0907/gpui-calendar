//! Rust tables and contrast math checked against the fixtures under tests/fixtures, generated from the approved palette.
use crate::theme::contrast::*;
use crate::theme::palette::*;
use serde_json::Value;

fn load(s: &str) -> Value {
    serde_json::from_str(s).expect("fixture parses")
}

fn palette() -> Value {
    load(include_str!("../../../tests/fixtures/palette_out.json"))
}

fn expected() -> Value {
    load(include_str!(
        "../../../tests/fixtures/contrast_expected.json"
    ))
}

fn hex(v: &Value) -> Rgb {
    Rgb::hex(v.as_str().expect("hex string"))
}

fn hue(v: &Value) -> u16 {
    u16::try_from(v.as_u64().expect("hue number")).unwrap()
}

#[test]
fn appearance_palette_consts_equal_json() {
    let p = palette();
    assert_eq!(p["accents"].as_object().unwrap().len(), ACCENTS.len());
    for a in ACCENTS {
        let j = &p["accents"][format!("{a:?}")];
        assert_eq!(accent(a).rgb, hex(&j["hex"]), "{a:?} hex");
        assert_eq!(accent(a).hue, hue(&j["hue"]), "{a:?} hue");
    }
    assert_eq!(p["slots"].as_object().unwrap().len(), SLOTS.len());
    for s in SLOTS {
        let (j, c) = (&p["slots"][format!("{s:?}")], slot(s));
        assert_eq!(c.hue, hue(&j["hue"]), "{s:?} hue");
        assert_eq!(
            (c.light.dot, c.light.text),
            (hex(&j["ld"]), hex(&j["lx"])),
            "{s:?} light"
        );
        assert_eq!(
            (c.light.tint.rgb, c.light.tint.alpha),
            (hex(&j["lt"]), 1.0),
            "{s:?} light tint"
        );
        assert_eq!(
            (c.dark.dot, c.dark.text),
            (hex(&j["dd"]), hex(&j["dx"])),
            "{s:?} dark"
        );
        let t: Vec<f64> = j["dt"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let rgb = Rgb(t[0] as u8, t[1] as u8, t[2] as u8);
        assert_eq!(
            (c.dark.tint.rgb, c.dark.tint.alpha),
            (rgb, t[3]),
            "{s:?} dark tint"
        );
    }
    assert_eq!(p["neutrals"].as_object().unwrap().len(), NEUTRALS.len());
    for n in NEUTRALS {
        let j = &p["neutrals"][format!("{n:?}")];
        for (th, dark) in [("light", false), ("dark", true)] {
            let (s, w) = (neutral(n, dark), &j[th]);
            let got = [
                s.page, s.surface, s.subtle, s.field, s.line, s.border, s.t1, s.t2, s.t3,
            ];
            let want = [
                "page", "surface", "subtle", "field", "line", "border", "t1", "t2", "t3",
            ]
            .map(|k| hex(&w[k]));
            assert_eq!(got, want, "{n:?} {th}");
        }
        let (b, w) = (banner(n), &j["banner"]);
        assert_eq!(
            [b.from, b.to, b.label, b.secondary],
            ["bn1", "bn2", "bnl", "bns"].map(|k| hex(&w[k])),
            "{n:?} banner"
        );
        assert_eq!(b.value, Rgb::WHITE);
    }
}

/// Every contrast the guide checks, keyed like `tests/fixtures/contrast_expected.json`.
fn measured() -> Vec<(String, f64)> {
    let mut v = Vec::new();
    let mut put = |k: String, fg: Rgb, bg: Rgb| v.push((k, contrast(fg, bg)));
    for n in NEUTRALS {
        for a in ACCENTS {
            put(
                format!("ad/{n:?}/{a:?}"),
                accent(a).rgb,
                neutral(n, true).surface,
            );
        }
        for (th, dark) in [("light", false), ("dark", true)] {
            let t = neutral(n, dark);
            for s in SLOTS {
                let sc = slot(s).scheme(dark);
                put(format!("ds/{n:?}/{th}/{s:?}"), sc.dot, t.surface);
                put(format!("dw/{n:?}/{th}/{s:?}"), sc.dot, t.subtle);
                put(
                    format!("tt/{n:?}/{th}/{s:?}"),
                    sc.text,
                    sc.tint.over(t.surface),
                );
            }
            for (sn, bg) in [
                ("page", t.page),
                ("surface", t.surface),
                ("subtle", t.subtle),
                ("field", t.field),
            ] {
                for (tn, fg) in [("t1", t.t1), ("t2", t.t2), ("t3", t.t3)] {
                    put(format!("{tn}/{n:?}/{th}/{sn}"), fg, bg);
                }
            }
            put(format!("bf/{n:?}/{th}"), t.border, t.field);
        }
        let b = banner(n);
        for (end, bg) in [("from", b.from), ("to", b.to)] {
            put(format!("bn/{n:?}/{end}/label"), b.label, bg);
            put(format!("bn/{n:?}/{end}/secondary"), b.secondary, bg);
            put(format!("bn/{n:?}/{end}/value"), b.value, bg);
        }
    }
    for a in ACCENTS {
        put(format!("aw/{a:?}"), Rgb::WHITE, accent(a).rgb);
    }
    v
}

#[test]
fn appearance_contrast_matches_python_reference() {
    let e = expected();
    let want = e["contrast"].as_object().unwrap();
    let got = measured();
    assert_eq!(got.len(), want.len(), "same set of checks as the reference");
    for (k, v) in got {
        let w = want
            .get(&k)
            .unwrap_or_else(|| panic!("{k} missing from reference"))
            .as_f64()
            .unwrap();
        assert!((v - w).abs() < 0.0005, "{k}: rust {v:.4} vs python {w:.4}");
    }
}

#[test]
fn appearance_over_ties_round_to_even_like_python() {
    let e = expected();
    let cases = e["over"].as_array().unwrap();
    assert!(cases.len() >= 3);
    for c in cases {
        let rgb = |i: usize| {
            let a = c[i].as_array().unwrap();
            Rgb(
                a[0].as_u64().unwrap() as u8,
                a[1].as_u64().unwrap() as u8,
                a[2].as_u64().unwrap() as u8,
            )
        };
        assert_eq!(over(rgb(0), c[1].as_f64().unwrap(), rgb(2)), rgb(3), "{c}");
    }
}

#[test]
fn appearance_fixture_enum_names_cover_json_keys() {
    // Guards the Debug-name lookups above: a renamed variant must not silently skip a JSON entry.
    let p = palette();
    for k in p["accents"].as_object().unwrap().keys() {
        assert!(ACCENTS.iter().any(|a| format!("{a:?}") == *k), "{k}");
    }
    for k in p["slots"].as_object().unwrap().keys() {
        assert!(SLOTS.iter().any(|a| format!("{a:?}") == *k), "{k}");
    }
}
