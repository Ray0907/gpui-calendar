//! Accessibility-tree audit: role, name and state of every element the kit test support observes, for month, week, day, year,
//! Appearance and the event popover. Interactive roles need a name, ambiguous names are listed, and states must match the model.
//! Writes artifacts/verify/ax-audit.txt. Harness-less (Metal), like the other pixel tests.
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
struct Row {
    id: String,
    role: String,
    label: String,
    checked: Option<bool>,
    selected: Option<bool>,
    expanded: Option<bool>,
    disabled: Option<bool>,
}

#[cfg(target_os = "macos")]
fn main() {
    use std::collections::BTreeMap;

    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::settings::{Accent, CalendarColor, Mode, Neutral};
    use gpui_calendar::store::{CALENDAR_NAMES, WORK};
    use gpui_calendar::theme::palette::{ACCENTS, NEUTRALS, SLOTS};
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestQueryExt, TestWindowExt};
    use gpui_kit::Role;
    use headless::*;

    // Roles a user operates: each must have a name. GridCell is listed too, it is the month cell.
    let interactive = |r: &str| ["Button", "CheckBox", "RadioButton", "Switch", "ComboBox", "TimeInput", "TextInput", "MultilineTextInput", "Tab", "MenuItem", "GridCell"].contains(&r);
    // Many elements of one kind are summarised in the artifact; they are still all checked.
    let group = ["month-cell-", "month-head-", "month-ev-", "month-more-", "year-day-", "year-month-", "year-ring-", "week-head-", "mini-day-", "minipick-month-"];
    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
    let events = || {
        let soon = day(28).and_hms_opt(19, 0, 0).unwrap();
        vec![Event::timed(12, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1)), Event::all_day(10, "Offsite", WORK, day(6), day(15))]
    };
    let mut out = String::new();
    let fails = std::cell::RefCell::new(Vec::<String>::new());
    let mut dup_report: Vec<String> = Vec::new();
    let mut total = 0usize;

    for name in ["month", "month-changed", "minipick", "week", "day", "year", "appearance", "appearance-changed", "popover", "popover-edited"] {
        let view = match name { "week" => View::Week, "day" => View::Day, "year" => View::Year, _ => View::Month };
        let mut win = open(Mode::Light, Neutral::Cool, view, events(), (1440., 900.));
        // Move the model away from its defaults so a state that ignores the model shows.
        let s = win.shell.clone();
        match name {
            "month-changed" => win.cx.update(|cx| s.update(cx, |s, cx| {
                s.model.toggle_calendar(1);
                s.model.pick_day(day(14));
                s.sync(cx);
            })),
            "appearance" | "appearance-changed" => win.cx.update(|cx| s.update(cx, |s, cx| {
                s.model.toggle_appearance();
                if name == "appearance-changed" {
                    s.model.set_mode(Mode::Dark);
                    s.model.set_neutral(Neutral::Warm);
                    let _ = s.model.set_accent(Accent::Blue);
                    let _ = s.model.set_calendar_color(2, CalendarColor::Violet);
                }
                s.sync(cx);
            })),
            "popover" | "popover-edited" => {
                win.cx.update_window(win.h, |_, w, cx| s.update(cx, |s, cx| s.show_event(12, w, cx))).unwrap();
            }
            _ => {}
        }
        win.render();
        if name == "minipick" {
            win.act(|w, cx| w.click("mini-title", cx));
        }
        if name == "popover-edited" {
            win.act(|w, cx| w.click("pop-cal-1", cx));
            win.act(|w, cx| w.click("pop-all-day", cx));
        }
        // The month cursor (aria selected) shows only while the grid has focus: focus it on the anchor's cell for these two.
        let grid_focus = matches!(name, "month" | "month-changed");
        if grid_focus {
            let anchor = win.cx.update(|cx| win.shell.read(cx).model.anchor);
            win.act(|w, cx| w.click(format!("month-cell-{anchor}"), cx));
        }
        let rows: Vec<Row> = win
            .cx
            .update_window(win.h, |_, w, _| {
                let all = w.elements();
                all.iter()
                    .filter(|e| e.role().is_some())
                    .map(|e| Row {
                        id: e.path().last().map(|p| format!("{p:?}").trim_start_matches("Name(\"").trim_end_matches("\")").to_string()).unwrap_or_default(),
                        role: format!("{:?}", e.role().unwrap()),
                        label: e.label().unwrap_or("").to_string(),
                        checked: e.checked(),
                        selected: e.selected(),
                        expanded: e.expanded(),
                        disabled: e.disabled(),
                    })
                    .collect()
            })
            .unwrap();
        total += rows.len();
        let get = |id: &str| rows.iter().find(|r| r.id == id);
        let bad = |msg: String| fails.borrow_mut().push(format!("[{name}] {msg}"));

        // 1. every interactive role has a name.
        // gpui-component's DatePicker names its trigger only by value (`aria_value`) and has no label setter: a known kit gap, the wrapping
        // group carries the name. Exactly two such triggers per popover are expected, so a kit fix or a third unnamed one shows. The time
        // fields are ours and carry their own name ("Start time, 09:30"): none is exempt.
        let kit_gap = |r: &Row| r.id.starts_with("NamedInteger(\"date-picker\"");
        for r in rows.iter().filter(|r| interactive(&r.role) && r.label.trim().is_empty() && !kit_gap(r)) {
            bad(format!("{} {} has no accessible name", r.role, r.id));
        }
        let gaps = rows.iter().filter(|r| kit_gap(r) && r.label.is_empty()).count();
        if gaps != if name.starts_with("popover") { 2 } else { 0 } {
            bad(format!("{gaps} unnamed kit date-picker triggers, expected 2 (the days) in the popover and none elsewhere"));
        }
        if name.starts_with("popover") {
            for field in ["Start date, ", "End date, "] {
                if !rows.iter().any(|r| r.role == "Group" && r.label.starts_with(field)) {
                    bad(format!("no group named {field:?} around the date picker"));
                }
            }
        }
        // 2. ambiguous names: the same role and name on two controls.
        let mut seen: BTreeMap<(String, String), Vec<&str>> = BTreeMap::new();
        for r in rows.iter().filter(|r| interactive(&r.role) && !r.label.is_empty()) {
            seen.entry((r.role.clone(), r.label.clone())).or_default().push(&r.id);
        }
        for ((role, label), ids) in seen.iter().filter(|(_, ids)| ids.len() > 1) {
            dup_report.push(format!("[{name}] {role} {label:?} x{}: {}", ids.len(), ids.iter().take(4).copied().collect::<Vec<_>>().join(", ")));
        }
        // 3. states against the model.
        let (view, anchor, visible, mini, appearance, colors) = win.cx.update(|cx| {
            let m = &win.shell.read(cx).model;
            (m.view, m.anchor, m.store.calendars.iter().map(|c| c.visible).collect::<Vec<_>>(), m.mini_days().into_iter().map(|d| (d.date, d.selected)).collect::<Vec<_>>(), m.store.settings.appearance, (0..3u8).map(|i| m.calendar_color(i)).collect::<Vec<_>>())
        });
        let want = |id: &str, field: &str, expected: Option<bool>, got: Option<Option<bool>>| match got {
            None => bad(format!("{id} is not in the tree")),
            Some(g) if g != expected => bad(format!("{id} {field} is {g:?}, the model says {expected:?}")),
            _ => {}
        };
        if !name.starts_with("appearance") {
            for v in View::ALL {
                let id = format!("view-{}", v.name().to_lowercase());
                want(&id, "checked", Some(v == view), get(&id).map(|r| r.checked));
            }
            for (i, &cal_visible) in visible.iter().enumerate() {
                let id = format!("calendar-row-{i}");
                want(&id, "checked", Some(cal_visible), get(&id).map(|r| r.checked));
            }
            for (date, selected) in mini {
                let id = format!("mini-day-{date}");
                want(&id, "checked", Some(selected), get(&id).map(|r| r.checked));
            }
            let picked: Vec<_> = rows.iter().filter(|r| r.id.starts_with("month-cell-") && r.selected == Some(true)).collect();
            // With the grid focused month shows exactly one selected cell, the anchor's; unfocused (and in other views) none.
            let expect: Vec<String> = if view == View::Month && grid_focus { vec![format!("month-cell-{anchor}")] } else { vec![] };
            if picked.iter().map(|r| r.id.clone()).collect::<Vec<_>>() != expect {
                bad(format!("selected month cells {:?}, expected {:?}", picked.iter().map(|r| &r.id).collect::<Vec<_>>(), expect));
            }
        }
        if name.starts_with("appearance") {
            // A swatch is disabled exactly when the model gives a reason, and its label then says so.
            let reasons = win.cx.update(|cx| ACCENTS.map(|x| (x, win.shell.read(cx).model.accent_reason(x).is_some())));
            for (x, off) in reasons {
                let id = format!("accent-{x:?}");
                let got = get(&id).map(|r| (r.disabled.unwrap_or(false), r.label.contains("unavailable")));
                if got != Some((off, off)) {
                    bad(format!("{id} (disabled, labelled unavailable) is {got:?}, the model says {off}"));
                }
            }
        }
        if name == "minipick" {
            // The title names the action and says it is expanded; each month is named with its month and year and is checked only for the view's month.
            want("mini-title", "expanded", Some(true), get("mini-title").map(|r| r.expanded));
            if get("mini-title").map(|r| r.label.as_str()) != Some("Choose month") {
                bad("mini-title is not named \"Choose month\"".into());
            }
            for m in 1..=12u32 {
                let id = format!("minipick-month-2026-{m:02}");
                want(&id, "checked", Some(m == chrono::Datelike::month(&anchor)), get(&id).map(|r| r.checked));
                let name = NaiveDate::from_ymd_opt(2026, m, 1).unwrap().format("%B 2026").to_string();
                if get(&id).map(|r| r.label.as_str()) != Some(name.as_str()) {
                    bad(format!("{id} is not named {name:?}"));
                }
            }
        }
        if name == "month-changed" {
            want("calendar-row-1", "checked", Some(false), get("calendar-row-1").map(|r| r.checked));
            want("mini-day-2026-10-14", "checked", Some(true), get("mini-day-2026-10-14").map(|r| r.checked));
            want("mini-day-2026-10-28", "checked", Some(false), get("mini-day-2026-10-28").map(|r| r.checked));
        }
        if name.starts_with("appearance") {
            let a = appearance;
            for (id, mode) in [("mode-light", Mode::Light), ("mode-dark", Mode::Dark), ("mode-system", Mode::System)] {
                want(id, "checked", Some(a.mode == mode), get(id).map(|r| r.checked));
            }
            for x in ACCENTS {
                let id = format!("accent-{x:?}");
                want(&id, "checked", Some(a.accent == x), get(&id).map(|r| r.checked));
            }
            for x in NEUTRALS {
                let id = format!("neutral-{x:?}");
                want(&id, "checked", Some(a.neutral == x), get(&id).map(|r| r.checked));
            }
            for i in 0..CALENDAR_NAMES.len() as u8 {
                for c in SLOTS {
                    let id = format!("calendar-color-{i}-{c:?}");
                    want(&id, "checked", Some(colors[usize::from(i)] == Some(c)), get(&id).map(|r| r.checked));
                }
            }
            if name == "appearance-changed" {
                want("mode-dark", "checked", Some(true), get("mode-dark").map(|r| r.checked));
                want("neutral-Warm", "checked", Some(true), get("neutral-Warm").map(|r| r.checked));
            }
        }
        if name.starts_with("popover") {
            let edited = name == "popover-edited";
            for i in 0..CALENDAR_NAMES.len() {
                let id = format!("pop-cal-{i}");
                want(&id, "checked", Some(i == usize::from(if edited { 1 } else { WORK })), get(&id).map(|r| r.checked));
            }
            want("pop-all-day", "checked", Some(edited), get("pop-all-day").map(|r| r.checked));
            for id in ["pop-save", "pop-cancel", "pop-delete"] {
                if get(id).is_none() {
                    bad(format!("{id} is not in the tree"));
                }
            }
            if !rows.iter().any(|r| r.role == format!("{:?}", Role::TextInput)) {
                bad("no text input in the popover".into());
            }
        }

        // Artifact: one line per element; grouped kinds as one line with counts.
        out += &format!("## {name} ({} elements with a role)\n{:<20} {:<42} {}\n", rows.len(), "role", "name", "state");
        let state = |r: &Row| [("checked", r.checked), ("selected", r.selected), ("expanded", r.expanded), ("disabled", r.disabled)].iter().filter_map(|(k, v)| v.map(|v| format!("{k}={v}"))).collect::<Vec<_>>().join(" ");
        let mut groups: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
        for r in &rows {
            match group.iter().find(|p| r.id.starts_with(**p)) {
                Some(p) => groups.entry(p).or_default().push(r),
                None => out += &format!("{:<20} {:<42} {}\n", r.role, r.label, state(r)),
            }
        }
        for (p, rs) in groups {
            let on = |f: fn(&Row) -> Option<bool>| rs.iter().filter(|r| f(r) == Some(true)).count();
            out += &format!("{:<20} {:<42} {} x{}, e.g. {:?}; checked {} selected {} unnamed {}\n", rs[0].role, format!("{p}*"), "group", rs.len(), rs[0].label, on(|r| r.checked), on(|r| r.selected), rs.iter().filter(|r| r.label.is_empty()).count());
        }
        out += "\n";
    }
    out += "## known kit gap: the two popover date-picker triggers (ComboBox) carry their value but no name; each sits in a Group named \"Start date, <day>\" / \"End date, <day>\"\n\n";
    out += &format!("## ambiguous names (same role and name on several controls): {}\n{}\n", dup_report.len(), dup_report.join("\n"));
    let fails = fails.into_inner();
    out += &format!("\n{total} elements, {} failing checks\n{}\n", fails.len(), fails.join("\n"));
    artifact("ax-audit.txt", &out);
    assert!(fails.is_empty(), "ax audit failures:\n{}", fails.join("\n"));
    println!("ax audit ok: {total} elements, {} ambiguous names listed", dup_report.len());
}
#[cfg(not(target_os = "macos"))]
fn main() {}
