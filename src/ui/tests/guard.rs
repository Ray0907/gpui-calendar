//! View code takes colors from the theme module only: no color literals or constructors under `src/ui`.
//! No file is exempt: fixed marks (white check, black scrims and shadows) live in `theme::gpui_adapter`.
use std::{fs, path::Path};

const CONSTRUCTORS: [&str; 19] = ["rgb(", "rgba(", "hsl(", "hsla(", "Rgb(", "Rgb::hex(", "white()", "black()", "red()", "green()", "blue()", "yellow()", "transparent_black()", "transparent_white()", "gray(", "grey(", "opaque_", "shade(", "on_color("];

/// `.opacity(` arguments the doc allows (design-tokens 2 and 3): press 85%, calendar time text 80%, disabled 50%,
/// `d`, the fade-in animation delta (not a fill), and `SWATCH_UNAVAILABLE`, the unavailable swatch's 35% (mock wins over the disabled 50%;
/// named, so a bare 0.35 stays rejected).
const OPACITIES: [&str; 5] = ["0.85", "0.8", "0.5", "d", "SWATCH_UNAVAILABLE"];

/// A struct literal of a color type: `Hsla { h: ..` or `Rgba{ r: ..`, not a `-> Hsla {` signature or an `impl .. for Hsla {`.
fn color_literal(line: &str) -> bool {
    ["Hsla", "Rgba"].iter().any(|ty| {
        line.match_indices(ty).any(|(i, _)| {
            let after = line[i + ty.len()..].trim_start();
            let starts_word = !line[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_');
            let field = after.strip_prefix('{').map(|b| b.trim_start()).is_some_and(|b| {
                let id = b.chars().take_while(|c| c.is_alphanumeric() || *c == '_').count();
                id > 0 && b[id..].trim_start().starts_with(':')
            });
            starts_word && field
        })
    })
}

/// `.opacity(x)` whose `x` the doc does not allow.
fn bad_opacity(line: &str) -> Option<String> {
    line.match_indices(".opacity(").map(|(i, k)| line[i + k.len()..].split(')').next().unwrap_or("").trim()).find(|a| !OPACITIES.contains(a)).map(str::to_string)
}

/// Files that may name colors: a `tests` directory is skipped by the walker; of the files, exactly `tests.rs` and `*_tests.rs`.
fn is_test_file(name: &str) -> bool {
    name == "tests.rs" || name.ends_with("_tests.rs")
}

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
    if color_literal(line) {
        return Some("color struct literal".into());
    }
    if let Some(a) = bad_opacity(line) {
        return Some(format!("opacity({a}) on a fill"));
    }
    // A name must start at a word boundary: `anchored()` is not `red()`.
    let starts_word = |k: &str| line.match_indices(k).any(|(i, _)| !line[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_'));
    CONSTRUCTORS.iter().find(|k| starts_word(k)).map(|k| format!("color constructor {k}"))
}

#[test]
fn detector_catches_every_literal_form_and_spares_attributes() {
    for bad in ["bg(\"#fff\")", "x = \"#1a2b3c\"", "\"#1A2B3C80\"", "rgb(0x1)", "rgba(1., 2., 3., 1.)", "hsla(0., 0., 0., 1.)", "hsl(1., 1., 1.)", ".bg(white())", "black().opacity(0.2)", "0xff00ff", "gpui_kit::gray(0.5)", "opaque_grey(0.5, 1.)", "bg(gpui_kit::red())", "transparent_black()", "x.blue()", "Rgb(1, 2, 3)", "Rgb::hex(\"#112233\")", "Rgb::hex(\"112233\")", "Hsla { h: 0., s: 0., l: 0., a: 1. }", "Rgba{ r: 1., g: 1., b: 1., a: 1. }", "c.opacity(0.4)", ".bg(x).opacity(0.9)", "x.opacity(alpha)", "s.opacity(0.35)", ".bg(shade(0.18))", "on_color()"] {
        assert!(violation(bad).is_some(), "should flag {bad}");
    }
    for ok in ["#[derive(Debug)]", "#![allow(x)]", "c(accent(ac).rgb)", "Rgb::new()", "// issue #12", "r#\"raw\"#", "use crate::theme::gpui_adapter::hsla;", "\"Tint of the grays\"", "let tired = 1;", "anchored().anchor(c)", "x.fired()", "to_rgb(1)", "fn c() -> Hsla {", "impl From<X> for Rgba {", "s.opacity(0.85)", "d.opacity(0.8)", "d.opacity(SWATCH_UNAVAILABLE)", "el.opacity(d)", "use gpui_kit::Rgb;", "banner_shade(1)", "x.nonon_color()"] {
        assert!(violation(ok).is_none(), "should allow {ok}");
    }
}

#[test]
fn only_tests_rs_and_suffix_tests_rs_are_test_files() {
    for yes in ["tests.rs", "popover_tests.rs", "x_tests.rs"] {
        assert!(is_test_file(yes), "{yes} is test code");
    }
    for no in ["contests.rs", "attests.rs", "tests_helper.rs", "shell.rs", "tests"] {
        assert!(!is_test_file(no), "{no} is view code");
    }
}

/// Every non-test `.rs` file under `dir`, subdirectories included. Test code (`tests/` folders, `*tests.rs`) may name colors.
fn view_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if p.is_dir() {
            if name != "tests" {
                view_files(&p, out);
            }
        } else if name.ends_with(".rs") && !is_test_file(&name) {
            out.push(p);
        }
    }
}

#[test]
fn guard_walks_subdirectories() {
    let mut files = Vec::new();
    view_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"), &mut files);
    for must in ["shell.rs", "popover/view.rs", "month/week.rs", "year/mod.rs", "week/blocks.rs"] {
        assert!(files.iter().any(|f| f.ends_with(must)), "{must} is not scanned");
    }
    assert!(files.iter().all(|f| !f.components().any(|c| c.as_os_str() == "tests")), "test code is scanned");
}

#[test]
fn no_color_literals_in_view_code() {
    let mut files = Vec::new();
    view_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"), &mut files);
    for p in files {
        for (i, l) in fs::read_to_string(&p).unwrap().lines().enumerate() {
            if let Some(why) = violation(l) {
                panic!("{}:{}: {why}: {l}", p.display(), i + 1);
            }
        }
    }
}
