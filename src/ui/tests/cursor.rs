//! Every clickable control shows the pointer hand. GPUI exposes the cursor to no test, so the rule is held at the source: a control goes through
//! `controls::control` / `control_disabled` or takes `.pointer()` / `.pointer_unless(..)` / an explicit `.cursor(..)` (event blocks: open and
//! closed hand), and nothing under `src/ui` calls `cursor_pointer` or `cursor_default` except the `Pointer` helper itself.
use std::{fs, path::Path};

/// Roles of something the pointer can press.
const PRESSABLE: [&str; 8] = ["Role::Button", "Role::CheckBox", "Role::Switch", "Role::RadioButton", "Role::ComboBox", "Role::ListBoxOption", "Role::Tab", "Role::MenuItem"];

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if p.is_dir() && name != "tests" {
            sources(&p, out);
        } else if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
            out.push((p.display().to_string(), fs::read_to_string(&p).unwrap()));
        }
    }
}

/// What is wrong with `file`, one line each.
fn problems(file: &str, text: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        if !file.ends_with("controls.rs") && (code.contains(".cursor_pointer(") || code.contains(".cursor_default(")) {
            bad.push(format!("{file}:{}: use `.pointer()` / `.pointer_unless(..)`, not the raw cursor method", i + 1));
        }
        if !PRESSABLE.iter().any(|r| code.contains(r)) || code.contains("control(") || code.contains("control_disabled(") {
            continue;
        }
        // The rest of the statement: up to its closing `;`, at most 40 lines.
        let end = lines[i..].iter().take(40).position(|l| l.trim_end().ends_with(';')).map_or(40, |n| n + 1);
        let chain = lines[i..(i + end).min(lines.len())].join("\n");
        if !(chain.contains(".pointer(") || chain.contains(".pointer_unless(") || chain.contains(".cursor(")) {
            bad.push(format!("{file}:{}: a pressable element built without `control` and without a pointer cursor", i + 1));
        }
    }
    bad
}

#[test]
fn the_detector_flags_an_unpointered_control_and_a_raw_cursor_call() {
    let bare = "let x = div().id(\"a\").role(Role::Button).child(\"A\");\n";
    assert_eq!(problems("src/ui/x.rs", bare).len(), 1);
    assert!(problems("src/ui/x.rs", "let x = div().role(Role::Button).pointer().child(\"A\");\n").is_empty());
    assert!(problems("src/ui/x.rs", "let x = control(\"a\", Role::Button, \"A\", g, r, w, cx);\n").is_empty());
    assert_eq!(problems("src/ui/x.rs", "let x = div().cursor_pointer();\n").len(), 1);
    assert!(problems("src/ui/controls.rs", "self.cursor_pointer()\n").is_empty());
}

#[test]
fn every_pressable_control_under_src_ui_has_a_pointer_cursor() {
    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"), &mut files);
    let bad: Vec<String> = files.iter().flat_map(|(f, t)| problems(f, t)).collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// The kit DatePicker popup is a deferred child that occludes hover, so no wrapper can set its cursor; the patched `gpui-component` calendar
/// item does, and only for enabled day cells and the prev/next buttons. The committed patch is the source of truth; `vendor/` (rebuilt by
/// `build/vendor-gpui-component.sh`, git-ignored) must exist and match.
fn guarded_once(text: &str, what: &str) {
    let at = text.find(".cursor_pointer()").unwrap_or_else(|| panic!("{what} has no cursor_pointer"));
    let before = &text[at.saturating_sub(500)..at];
    for guard in ["!state.is_disabled()", "!state.is_muted()", "CalendarItemKind::Previous", "CalendarItemKind::Next", "CalendarItemKind::Day"] {
        assert!(before.contains(guard), "{what}: cursor_pointer is not guarded by {guard}");
    }
    assert_eq!(text.matches(".cursor_pointer()").count(), 1, "{what}");
}

#[test]
fn the_calendar_patch_points_enabled_days_and_arrows_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    guarded_once(&fs::read_to_string(root.join("patches/gpui-component-0.8.1-calendar-cursor.patch")).unwrap(), "patch");
    let vendored = root.join("vendor/gpui-component/src/time/calendar.rs");
    assert!(vendored.exists(), "vendor/ is missing: run build/vendor-gpui-component.sh");
    guarded_once(&fs::read_to_string(vendored).unwrap(), "vendor calendar.rs");
}
