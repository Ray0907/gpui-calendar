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

/// `(name, inline value)` of every entry in every `[*dependencies*]` table of a Cargo.toml.
/// The `[dependencies.foo]` table form has an empty value.
fn dependency_entries(cargo: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut in_deps = false;
    for line in cargo.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some(header) = line.strip_prefix('[') {
            let header = header.trim_end_matches(']');
            in_deps = header.ends_with("dependencies") || header.contains("dependencies.");
            if let Some((_, name)) = header.split_once("dependencies.") {
                entries.push((name.to_string(), String::new())); // [dependencies.foo]
                in_deps = false;
            }
        } else if in_deps && let Some((name, value)) = line.split_once('=') {
            entries.push((name.trim().to_string(), value.split_whitespace().collect()));
        }
    }
    entries
}

fn dependency_names(cargo: &str) -> Vec<String> {
    dependency_entries(cargo).into_iter().map(|(n, _)| n).collect()
}

/// Dependencies outside the allowed set. The package itself is allowed only as the dev-dependency
/// that turns on its own `test-hooks` feature for tests: path `.`, that one feature, nothing else.
fn disallowed_deps(cargo: &str) -> Vec<String> {
    let me = ui_token() + "-calendar";
    dependency_entries(cargo)
        .into_iter()
        .filter(|(n, v)| if *n == me { v != r#"{path=".",features=["test-hooks"]}"# } else { !allowed_deps().contains(n) })
        .map(|(n, _)| n)
        .collect()
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
    assert_eq!(disallowed_deps(&cargo), Vec::<String>::new());
}

#[test]
fn self_dependency_is_allowed_only_as_the_test_hooks_path() {
    let me = ui_token() + "-calendar";
    let with = |dep: &str| format!("[dependencies]\nserde = \"1\"\n[dev-dependencies]\n{dep}\n");
    let ok = format!(r#"{me} = {{ path = ".", features = ["test-hooks"] }}"#);
    assert!(disallowed_deps(&with(&ok)).is_empty());
    for bad in [
        "evil = \"1\"".to_string(),
        format!(r#"{me} = {{ path = ".", features = ["other"] }}"#),
        format!(r#"{me} = {{ path = ".", features = ["test-hooks", "other"] }}"#),
        format!(r#"{me} = {{ path = "../elsewhere", features = ["test-hooks"] }}"#),
        format!(r#"{me} = {{ path = ".", features = ["test-hooks"], default-features = false }}"#),
        format!(r#"{me} = {{ version = "1", features = ["test-hooks"] }}"#),
    ] {
        assert_eq!(disallowed_deps(&with(&bad)).len(), 1, "{bad}");
    }
    assert_eq!(disallowed_deps(&format!("[dev-dependencies.{me}]\npath = \".\"\n")).len(), 1);
}
