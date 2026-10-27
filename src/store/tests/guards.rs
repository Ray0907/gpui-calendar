use super::*;

// The tokens are built at run time so this file does not trip its own scan.
fn ui_token() -> String {
    ["gp", "ui"].concat()
}

/// True if `src` mentions the UI crate in any form (`use`, a `::` path, an underscore or dash
/// variant of the name, `extern crate`). Only the crate's own name `gpui-calendar` is allowed.
fn mentions_ui_crate(src: &str) -> bool {
    src.replace(&format!("{}-calendar", ui_token()), "").contains(&ui_token())
}

/// Dependency names in every `[*dependencies*]` table of a Cargo.toml.
fn dependency_names(cargo: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in cargo.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some(header) = line.strip_prefix('[') {
            let header = header.trim_end_matches(']');
            in_deps = header.ends_with("dependencies") || header.contains("dependencies.");
            if let Some((_, name)) = header.split_once("dependencies.") {
                names.push(name.to_string()); // [dependencies.foo]
                in_deps = false;
            }
        } else if in_deps && let Some((name, _)) = line.split_once('=') {
            names.push(name.trim().to_string());
        }
    }
    names
}

fn allowed_deps() -> [String; 5] {
    ["serde", "serde_json", "chrono", "time"].map(String::from).into_iter().chain([ui_token() + "-kit"]).collect::<Vec<_>>().try_into().unwrap()
}

#[test]
fn ui_token_scan_catches_every_form() {
    let t = ui_token();
    for bad in [
        format!("use {t}::Pixels;"),
        format!("fn f(_: {t}::Pixels) {{}}"),
        format!("let x = {t}_kit::px(1.);"),
        format!("extern crate {t};"),
        format!("use {t}_kit::{{div}};"),
        format!("// {t}"),
    ] {
        assert!(mentions_ui_crate(&bad), "{bad}");
    }
    for ok in [format!("const APP: &str = \"{t}-calendar\";"), "use chrono::Duration;".to_string()] {
        assert!(!mentions_ui_crate(&ok), "{ok}");
    }
}

#[test]
fn dependency_scan_reads_every_table_form() {
    let cargo = "[package]\nname = \"x\"\n[dependencies]\nserde = \"1\"\nevil = { version = \"1\" }\n[dev-dependencies]\nbad2 = \"1\"\n[target.'cfg(unix)'.dependencies]\nbad3 = \"1\"\n[dependencies.bad4]\nversion = \"1\"\n[[test]]\nname = \"screenshot\"\n";
    assert_eq!(dependency_names(cargo), ["serde", "evil", "bad2", "bad3", "bad4"]);
}

#[test]
fn store_and_domain_do_not_use_the_ui_crate() {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&root.join("store"), &mut files);
    walk(&root.join("domain"), &mut files);
    assert!(files.len() >= 4);
    for f in files {
        for (i, line) in fs::read_to_string(&f).unwrap().lines().enumerate() {
            assert!(!mentions_ui_crate(line), "{}:{}: {line}", f.display(), i + 1);
        }
    }
}

#[test]
fn cargo_dependencies_stay_within_the_allowed_set() {
    let cargo = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let names = dependency_names(&cargo);
    assert!(names.iter().any(|n| n == "serde"), "scan found no dependencies: {names:?}");
    for n in names {
        assert!(allowed_deps().contains(&n), "dependency {n} is not allowed");
    }
}
