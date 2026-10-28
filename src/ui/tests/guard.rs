//! View code takes colors from the theme module only: no color literals or constructors under `src/ui`.
//! No file is exempt: fixed marks (white check, black scrims and shadows) live in `theme::gpui_adapter`.
use std::{fs, path::Path};

const CONSTRUCTORS: [&str; 6] = ["rgb(", "rgba(", "hsl(", "hsla(", "white()", "black()"];

/// Why a line breaks the rule, if it does.
fn violation(line: &str) -> Option<String> {
    if line.contains("0x") {
        return Some("0x literal".into());
    }
    for r in line.split('#').skip(1) {
        let run = r.chars().take_while(char::is_ascii_hexdigit).count();
        if matches!(run, 3 | 4 | 6 | 8) {
            return Some("hex color".into());
        }
    }
    CONSTRUCTORS.iter().find(|k| line.contains(**k)).map(|k| format!("color constructor {k}"))
}

#[test]
fn detector_catches_every_literal_form_and_spares_attributes() {
    for bad in ["bg(\"#fff\")", "x = \"#1a2b3c\"", "\"#1A2B3C80\"", "rgb(0x1)", "rgba(1., 2., 3., 1.)", "hsla(0., 0., 0., 1.)", "hsl(1., 1., 1.)", ".bg(white())", "black().opacity(0.2)", "0xff00ff"] {
        assert!(violation(bad).is_some(), "should flag {bad}");
    }
    for ok in ["#[derive(Debug)]", "#![allow(x)]", "c(accent(ac).rgb)", "Rgb::new()", "// issue #12", "r#\"raw\"#", "use crate::theme::gpui_adapter::hsla;"] {
        assert!(violation(ok).is_none(), "should allow {ok}");
    }
}

#[test]
fn no_color_literals_in_view_code() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    for e in fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        for (i, l) in fs::read_to_string(&p).unwrap().lines().enumerate() {
            if let Some(why) = violation(l) {
                panic!("{}:{}: {why}: {l}", p.display(), i + 1);
            }
        }
    }
}
