//! Headless tests of the global shortcuts and the accessible-name audit.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Entity, Role, TestAppContext};
use std::{cell::Cell, rc::Rc};

fn open(cx: &mut TestAppContext, name: &str) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

fn key(cx: &mut TestAppContext, h: AnyWindowHandle, k: &str) {
    cx.update_window(h, |_, w, cx| w.key_down(k, false, cx)).unwrap();
}

fn view(cx: &mut TestAppContext, shell: &Entity<Shell>) -> View {
    cx.update(|cx| shell.read(cx).model.view)
}

fn anchor(cx: &mut TestAppContext, shell: &Entity<Shell>) -> chrono::NaiveDate {
    cx.update(|cx| shell.read(cx).model.anchor)
}

/// No calendar is on: no event to land on, and none can be created.
fn hide_all_calendars(cx: &mut TestAppContext, shell: &Entity<Shell>) {
    cx.update(|cx| {
        shell.update(cx, |s, _| {
            for id in s.model.store.calendars.iter().map(|c| c.id).collect::<Vec<_>>() {
                if s.model.new_event_calendar().is_some() {
                    s.model.toggle_calendar(id);
                }
            }
        })
    });
}

fn popover_open(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    cx.update(|cx| shell.read(cx).popover_by_key().is_some())
}

#[gpui_kit::test]
fn d_w_m_y_switch_the_view(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-views");
    for (k, v) in [("d", View::Day), ("w", View::Week), ("y", View::Year), ("m", View::Month)] {
        key(cx, h, k);
        assert_eq!(view(cx, &shell), v, "{k}");
    }
}

#[gpui_kit::test]
fn t_returns_to_today(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-today");
    let today = anchor(cx, &shell);
    key(cx, h, "right");
    assert_ne!(anchor(cx, &shell), today);
    key(cx, h, "t");
    assert_eq!(anchor(cx, &shell), today);
}

#[gpui_kit::test]
fn left_and_right_step_by_the_view_unit(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-step");
    key(cx, h, "w");
    let start = anchor(cx, &shell);
    key(cx, h, "right");
    assert_eq!(anchor(cx, &shell), start + chrono::Duration::days(7));
    key(cx, h, "left");
    key(cx, h, "left");
    assert_eq!(anchor(cx, &shell), start - chrono::Duration::days(7));
}

#[gpui_kit::test]
fn a_grid_that_uses_the_arrow_keeps_it(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-grid");
    key(cx, h, "y"); // the year grid always takes the arrows to move its cursor
    cx.update_window(h, |_, w, cx| {
        shell.read(cx).grid_focus().focus(w, cx);
        w.render_frame(cx);
    })
    .unwrap();
    let year = cx.update(|cx| shell.read(cx).model.shown_year());
    key(cx, h, "right");
    assert_eq!(cx.update(|cx| shell.read(cx).model.shown_year()), year, "moved the cursor, not the year");
}

#[gpui_kit::test]
fn n_opens_a_new_event_at_once(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-new");
    assert!(!popover_open(cx, &shell));
    key(cx, h, "n");
    assert_eq!(cx.update(|cx| shell.read(cx).popover_by_key()), Some(true), "opened by key: no animation");
    key(cx, h, "escape");
    assert!(!popover_open(cx, &shell), "Esc closes it");
}

#[gpui_kit::test]
fn shortcuts_stay_silent_while_the_popover_has_focus(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-field");
    key(cx, h, "n");
    let before = (view(cx, &shell), anchor(cx, &shell));
    for k in ["m", "w", "d", "y", "t", "n", "right", "left"] {
        key(cx, h, k);
    }
    assert_eq!((view(cx, &shell), anchor(cx, &shell)), before);
    assert!(popover_open(cx, &shell), "still the same open form");
}

#[gpui_kit::test]
fn shortcuts_with_a_modifier_belong_to_the_system(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-mod");
    // The helper parses "cmd-w" as Cmd held plus the key W, and the bare key does act: so the silence
    // below comes from the modifier check, not from an unknown key.
    assert!(gpui_kit::Keystroke::parse("cmd-w").unwrap().modifiers.modified());
    key(cx, h, "w");
    assert_eq!(view(cx, &shell), View::Week, "the bare key acts");
    key(cx, h, "m");
    for k in ["cmd-w", "ctrl-d", "alt-y", "cmd-n", "cmd-t"] {
        key(cx, h, k);
    }
    assert_eq!(view(cx, &shell), View::Month);
    assert!(!popover_open(cx, &shell));
}

#[gpui_kit::test]
fn shortcuts_survive_the_focused_grid_going_away(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-restore");
    cx.update_window(h, |_, w, cx| shell.read(cx).grid_focus().focus(w, cx)).unwrap();
    key(cx, h, "y"); // the month grid, which had focus, leaves the tree
    key(cx, h, "w");
    assert_eq!(view(cx, &shell), View::Week);
}

#[gpui_kit::test]
fn shortcuts_work_again_after_leaving_the_appearance_screen(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-back");
    cx.update_window(h, |_, w, cx| w.click("account-menu", cx)).unwrap();
    cx.update_window(h, |_, w, cx| w.click("account-menu", cx)).unwrap();
    key(cx, h, "w");
    assert_eq!(view(cx, &shell), View::Week);
}

#[gpui_kit::test]
fn n_with_every_calendar_hidden_says_why_and_is_not_swallowed(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-none");
    hide_all_calendars(cx, &shell);
    assert!(cx.update(|cx| shell.read(cx).model.new_event_calendar().is_none()));
    key(cx, h, "n");
    assert!(!popover_open(cx, &shell));
    assert!(cx.update(|cx| shell.read(cx).model.notice.contains("calendar")), "a visible reason");
}

fn focus_grid(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>) {
    cx.update_window(h, |_, w, cx| {
        shell.read(cx).grid_focus().focus(w, cx);
        w.render_frame(cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn at_the_grid_edge_the_arrow_steps_the_period(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-grids");
    hide_all_calendars(cx, &shell); // no event to move to: every arrow is at the edge
    for (k, unit) in [("m", chrono::Months::new(1)), ("w", chrono::Months::new(0)), ("d", chrono::Months::new(0))] {
        key(cx, h, k);
        focus_grid(cx, h, &shell);
        let before = anchor(cx, &shell);
        let next = match k {
            "m" => before.checked_add_months(unit).unwrap(),
            "w" => before + chrono::Duration::days(7),
            _ => before + chrono::Duration::days(1),
        };
        key(cx, h, "right");
        assert_eq!(anchor(cx, &shell), next, "{k}: right at the edge steps forward");
        key(cx, h, "left");
        assert_eq!(anchor(cx, &shell), before, "{k}: left steps back");
        key(cx, h, "down");
        assert_eq!(anchor(cx, &shell), before, "{k}: vertical arrows never step");
    }
}

#[gpui_kit::test]
fn inside_the_grid_the_arrow_moves_the_cursor_before_it_steps(cx: &mut TestAppContext) {
    for (name, v) in [("kb-edge-m", View::Month), ("kb-edge-w", View::Week)] {
        let event = gpui_calendar::domain::event::Event::timed(1, "Standup", gpui_calendar::store::WORK, at(28, 9, 0), at(28, 10, 0));
        let (h, shell) = common::open_week(cx, name, v, vec![event]);
        focus_grid(cx, h, &shell);
        let before = anchor(cx, &shell);
        key(cx, h, "right");
        assert_eq!(anchor(cx, &shell), before, "{name}: the first arrow selects the event");
        key(cx, h, "right");
        assert_ne!(anchor(cx, &shell), before, "{name}: no event further on: it steps");
    }
}

#[gpui_kit::test]
fn the_month_edge_is_the_last_event_of_the_shown_month(cx: &mut TestAppContext) {
    use gpui_calendar::domain::event::Event;
    let (a, b) = (at(28, 9, 0), at(28, 9, 0) + chrono::Months::new(1));
    let events = vec![Event::timed(1, "Now", gpui_calendar::store::WORK, a, a + chrono::Duration::hours(1)), Event::timed(2, "Later", gpui_calendar::store::WORK, b, b + chrono::Duration::hours(1))];
    let (h, shell) = common::open_week(cx, "kb-edge-month", View::Month, events);
    focus_grid(cx, h, &shell);
    let before = anchor(cx, &shell);
    key(cx, h, "right");
    assert_eq!(anchor(cx, &shell), before, "the first arrow selects the last event of this month");
    key(cx, h, "right");
    assert_eq!(anchor(cx, &shell), before.checked_add_months(chrono::Months::new(1)).unwrap(), "past it the month steps, not into the next month's events");
}

#[gpui_kit::test]
fn shortcuts_do_nothing_behind_the_appearance_screen(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-appearance");
    cx.update_window(h, |_, w, cx| w.click("account-menu", cx)).unwrap();
    key(cx, h, "w");
    key(cx, h, "n");
    assert_eq!(view(cx, &shell), View::Month);
    assert!(!popover_open(cx, &shell));
}

/// Roles a user can act on: each must be named.
const ACTIONABLE: [Role; 7] = [Role::Button, Role::CheckBox, Role::RadioButton, Role::Switch, Role::TextInput, Role::Link, Role::Tab];

/// Every actionable or focus-tracked element, and those of them with no name: (seen, via a named ancestor, unnamed).
/// gpui-kit's `Input` puts its label on its own accessibility node, which the headless observation does not
/// record; the labelled field group around it is the name the test can see. The same goes for a date
/// picker's combo box, which gpui-kit gives no label of its own.
fn audit(cx: &mut TestAppContext, h: AnyWindowHandle) -> (usize, Vec<String>, Vec<String>) {
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let all = w.elements();
        let named = |e: &gpui_kit::test::ElementSnapshot| e.label().is_some_and(|l| !l.trim().is_empty());
        let acts: Vec<_> = all.iter().filter(|e| e.role().is_some_and(|r| ACTIONABLE.contains(&r)) || e.focused().is_some()).collect();
        let (mut inherited, mut bad) = (Vec::new(), Vec::new());
        for e in acts.iter().filter(|e| !named(e)) {
            let parent = all.iter().find(|a| a.path().len() < e.path().len() && e.path().starts_with(a.path()) && named(a) && a.role() == Some(Role::Group));
            let text = format!("{:?}: {:?}", e.path().last(), e.role());
            match (e.role(), parent) {
                (None | Some(Role::ComboBox | Role::TimeInput), Some(a)) => inherited.push(format!("{text} <- {}", a.label().unwrap_or_default())),
                _ => bad.push(text),
            }
        }
        (acts.len(), inherited, bad)
    })
    .unwrap()
}

#[gpui_kit::test]
fn no_interactive_control_lacks_an_accessible_name(cx: &mut TestAppContext) {
    let (h, _) = open(cx, "kb-audit");
    let mut seen = 0;
    for step in ["month", "w", "d", "y", "m", "appearance", "popover"] {
        match step {
            "month" => {}
            "appearance" => cx.update_window(h, |_, w, cx| w.click("account-menu", cx)).unwrap(),
            "popover" => {
                cx.update_window(h, |_, w, cx| w.click("account-menu", cx)).unwrap(); // back to the calendar
                key(cx, h, "n");
            }
            k => key(cx, h, k),
        }
        let (n, inherited, bad) = audit(cx, h);
        assert!(n > 0, "{step}: the audit saw nothing");
        assert!(bad.is_empty(), "{step}: unnamed controls: {bad:#?}");
        eprintln!("{step}: {n} controls, named by their field group: {inherited:?}");
        seen += n;
    }
    eprintln!("audited {seen} control observations");
}

#[gpui_kit::test]
fn each_grid_is_one_tab_stop(cx: &mut TestAppContext) {
    let (h, _) = open(cx, "kb-tabstop");
    for (k, grid) in [("m", "month-scroll"), ("w", "week-scroll"), ("y", "year-scroll")] {
        key(cx, h, k);
        cx.update_window(h, |_, w, cx| {
            w.render_frame(cx);
            let inside = w.elements().into_iter().filter(|e| e.focused().is_some() && e.path().iter().any(|p| format!("{p:?}").contains(grid))).count();
            assert_eq!(inside, 1, "{grid}: the grid itself is the only focus stop inside it");
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn with_reduced_motion_the_popover_still_opens_by_pointer_and_key(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-reduce");
    cx.update(|cx| cx.set_reduce_motion(true));
    assert!(cx.update(|cx| cx.reduce_motion()));
    cx.update_window(h, |_, w, cx| w.click("add-event", cx)).unwrap();
    assert!(popover_open(cx, &shell));
    key(cx, h, "escape");
    key(cx, h, "n");
    assert_eq!(cx.update(|cx| shell.read(cx).popover_by_key()), Some(true));
}

#[gpui_kit::test]
fn the_arrows_on_the_view_switch_do_not_step_the_period(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-segmented");
    cx.update_window(h, |_, w, cx| w.click("view-month", cx)).unwrap();
    let focused = cx.update_window(h, |_, w, cx| w.focused(cx)).unwrap();
    assert!(focused.is_some_and(|f| f != cx.update(|cx| shell.read(cx).grid_focus())), "a header control has focus");
    let before = anchor(cx, &shell);
    key(cx, h, "right");
    key(cx, h, "left");
    key(cx, h, "left");
    assert_eq!(anchor(cx, &shell), before);
}

#[gpui_kit::test]
fn shortcuts_survive_any_focused_element_being_unmounted(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-unmount");
    cx.update(|cx| shell.update(cx, |s, cx| { s.model.warn("Something to read".into()); s.sync(cx); }));
    cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
    // Dismissing takes the focused button out of the tree; view, screen, popover and toast stay as they were.
    cx.update_window(h, |_, w, cx| w.click("notice-dismiss", cx)).unwrap();
    cx.update_window(h, |_, w, cx| (0..2).for_each(|_| w.render_frame(cx))).unwrap();
    key(cx, h, "w");
    assert_eq!(view(cx, &shell), View::Week);
}

#[gpui_kit::test]
fn a_refused_new_event_warns_once_per_press(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-once");
    hide_all_calendars(cx, &shell);
    cx.update_window(h, |_, w, cx| w.key_down("n", false, cx)).unwrap();
    assert!(cx.update(|cx| shell.read(cx).model.notice.contains("calendar")));
    cx.update(|cx| shell.update(cx, |s, cx| { s.model.dismiss_notice(); s.sync(cx); }));
    for _ in 0..3 {
        cx.update_window(h, |_, w, cx| w.key_down("n", true, cx)).unwrap(); // the key held down
    }
    assert!(cx.update(|cx| shell.read(cx).model.notice.is_empty()), "repeats of a held key do not warn again");
}

/// What a user sees after a navigation: the view, the anchor and the title the header shows.
fn seen(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>) -> (View, chrono::NaiveDate, Option<String>) {
    let title = cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.find("header-title").label().map(|l| l.to_string())
    })
    .unwrap();
    (view(cx, shell), anchor(cx, shell), title)
}

#[gpui_kit::test]
fn each_key_and_its_header_button_end_in_the_same_state(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "kb-same-keys");
    let pairs = [("right", "nav-next"), ("left", "nav-prev"), ("t", "nav-today"), ("w", "view-week"), ("d", "view-day"), ("y", "view-year"), ("m", "view-month")];
    // The known place: the Month view on today (Today is clicked last so the focus is not on the view switch).
    let reset = |cx: &mut TestAppContext| {
        for id in ["view-month", "nav-today"] {
            cx.update_window(h, |_, w, cx| w.click(id, cx)).unwrap();
        }
    };
    for (k, id) in pairs {
        reset(cx);
        key(cx, h, "right"); // somewhere other than today, so "t" has something to undo
        key(cx, h, k);
        let by_key = seen(cx, h, &shell);
        reset(cx);
        key(cx, h, "right");
        cx.update_window(h, |_, w, cx| w.click(id, cx)).unwrap();
        assert_eq!(seen(cx, h, &shell), by_key, "{k} vs {id}");
    }
}
