//! The mini calendar's month picker: opening, picking, years, Escape, an outside click, the keyboard, and the card staying inside the window.
mod common;

use std::{cell::Cell, rc::Rc};

use chrono::NaiveDate;
use common::{at, open_sized, tmp};
use gpui_calendar::ui::Shell;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, Role, TestAppContext, px};

fn open(cx: &mut TestAppContext, name: &str, size: (f32, f32)) -> (AnyWindowHandle, Entity<Shell>) {
    let (h, shell) = open_sized(cx, &tmp(name), size, Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    frames(cx, h);
    (h, shell)
}

fn frames(cx: &mut TestAppContext, h: AnyWindowHandle) {
    cx.update_window(h, |_, w, cx| (0..4).for_each(|_| w.render_frame(cx))).unwrap();
}

fn click(cx: &mut TestAppContext, h: AnyWindowHandle, id: &str) {
    cx.update_window(h, |_, w, cx| w.click(id.to_string(), cx)).unwrap();
    frames(cx, h);
}

fn key(cx: &mut TestAppContext, h: AnyWindowHandle, k: &str) {
    cx.update_window(h, |_, w, cx| w.press(k, cx)).unwrap();
    frames(cx, h);
}

fn is_open(cx: &mut TestAppContext, shell: &Entity<Shell>) -> bool {
    cx.update(|cx| shell.read(cx).minipick_open())
}

fn state(cx: &mut TestAppContext, shell: &Entity<Shell>) -> (NaiveDate, String) {
    cx.update(|cx| (shell.read(cx).model.anchor, shell.read(cx).model.mini_title()))
}

fn title_focused(cx: &mut TestAppContext, h: AnyWindowHandle, shell: &Entity<Shell>) -> bool {
    let focus = cx.update(|cx| shell.read(cx).minipick_title_focus());
    cx.update_window(h, |_, w, _| focus.is_focused(w)).unwrap()
}

fn exists(cx: &mut TestAppContext, h: AnyWindowHandle, id: &str) -> bool {
    cx.update_window(h, |_, w, _| w.try_find(id.to_string()).is_some()).unwrap()
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

#[gpui_kit::test]
fn the_title_opens_a_card_of_twelve_named_months(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-open", (1280., 900.));
    assert!(!is_open(cx, &shell) && !exists(cx, h, "minipick"));
    click(cx, h, "mini-title");
    assert!(is_open(cx, &shell) && exists(cx, h, "minipick"));
    cx.update_window(h, |_, w, _| {
        assert_eq!(w.find("mini-title").role(), Some(Role::Button));
        assert_eq!(w.find("mini-title").label(), Some("Choose month"));
        assert_eq!(w.find("mini-title").expanded(), Some(true));
        for m in 1..=12 {
            let cell = w.find(format!("minipick-month-2026-{m:02}"));
            let name = day(2026, m, 1).format("%B 2026").to_string();
            assert_eq!(cell.label(), Some(name.as_str()));
            assert_eq!(cell.checked(), Some(m == 10), "only the month the view is on is selected: {name}");
        }
        assert_eq!(w.find("minipick-month-2026-10").description(), Some("current month"));
    })
    .unwrap();
    assert_eq!(cx.update(|cx| shell.read(cx).minipick_cursor()), (2026, 10));
}

#[gpui_kit::test]
fn picking_a_month_moves_the_mini_calendar_and_the_view(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-pick", (1280., 900.));
    click(cx, h, "mini-title");
    click(cx, h, "minipick-month-2026-03");
    assert_eq!(state(cx, &shell), (day(2026, 3, 28), "March 2026".into()));
    assert!(!is_open(cx, &shell) && !exists(cx, h, "minipick"));
    assert!(title_focused(cx, h, &shell), "focus goes back to the title");
    // The main view shows March too.
    cx.update_window(h, |_, w, _| assert!(w.try_find("month-cell-2026-03-28".to_string()).is_some())).unwrap();
}

#[gpui_kit::test]
fn the_year_arrows_reach_other_years(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-year", (1280., 900.));
    click(cx, h, "mini-title");
    click(cx, h, "minipick-next-year");
    click(cx, h, "minipick-next-year");
    assert!(exists(cx, h, "minipick-month-2028-10") && !exists(cx, h, "minipick-month-2026-10"));
    assert_eq!(state(cx, &shell).1, "October 2026", "browsing years changes nothing");
    click(cx, h, "minipick-prev-year");
    click(cx, h, "minipick-month-2027-05");
    assert_eq!(state(cx, &shell), (day(2027, 5, 28), "May 2027".into()));
}

#[gpui_kit::test]
fn escape_closes_without_a_change_and_returns_focus_to_the_title(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-esc", (1280., 900.));
    let before = state(cx, &shell);
    click(cx, h, "mini-title");
    click(cx, h, "minipick-next-year");
    key(cx, h, "escape");
    assert!(!is_open(cx, &shell));
    assert_eq!(state(cx, &shell), before);
    assert!(title_focused(cx, h, &shell));
    click(cx, h, "mini-title");
    assert_eq!(cx.update(|cx| shell.read(cx).minipick_cursor()), (2026, 10), "it opens on the month the mini calendar shows, not where it was left");
}

#[gpui_kit::test]
fn a_click_outside_closes_without_a_change(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-out", (1280., 900.));
    let before = state(cx, &shell);
    for outside in ["calendar-row-0", "month-cell-2026-10-20", "add-event", "view-week"] {
        click(cx, h, "mini-title");
        assert!(is_open(cx, &shell));
        click(cx, h, outside);
        assert!(!is_open(cx, &shell), "{outside}");
        assert_eq!(state(cx, &shell), before, "{outside} was not pressed through the backdrop");
        assert!(!cx.update(|cx| shell.read(cx).popover_by_key().is_some()), "{outside}");
    }
}

#[gpui_kit::test]
fn the_keyboard_opens_moves_picks_and_closes(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-keys", (1280., 900.));
    let focus = cx.update(|cx| shell.read(cx).minipick_title_focus());
    for opener in ["enter", "space"] {
        cx.update_window(h, |_, w, cx| focus.focus(w, cx)).unwrap();
        frames(cx, h);
        key(cx, h, opener);
        assert!(is_open(cx, &shell), "{opener} on the title opens the picker");
        key(cx, h, "escape");
        assert!(!is_open(cx, &shell) && title_focused(cx, h, &shell), "{opener}: Escape closes and the title has the focus");
    }
    cx.update_window(h, |_, w, cx| focus.focus(w, cx)).unwrap();
    key(cx, h, "enter");
    let cursor = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).minipick_cursor());
    for (k, want) in [("right", (2026, 11)), ("right", (2026, 12)), ("right", (2027, 1)), ("down", (2027, 4)), ("left", (2027, 3)), ("up", (2026, 12)), ("pagedown", (2027, 12)), ("pageup", (2026, 12))] {
        key(cx, h, k);
        assert_eq!(cursor(cx), want, "{k}");
    }
    assert_eq!(state(cx, &shell).1, "October 2026", "moving the keyboard changes nothing");
    key(cx, h, "left");
    key(cx, h, "enter");
    assert!(!is_open(cx, &shell));
    assert_eq!(state(cx, &shell), (day(2026, 11, 28), "November 2026".into()));
    assert!(title_focused(cx, h, &shell));
}

/// Focus outside the card, anchor not on today: the letter shortcuts must not fire behind the open picker.
#[gpui_kit::test]
fn the_shortcuts_do_not_fire_while_the_picker_is_open(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-block", (1280., 900.));
    click(cx, h, "mini-day-2026-10-20");
    click(cx, h, "mini-title");
    let view = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).model.view);
    let before = (state(cx, &shell), view(cx));
    assert_ne!(before.0.0, day(2026, 10, 28), "the anchor is not today");
    let root = cx.update(|cx| shell.read(cx).grid_focus());
    cx.update_window(h, |_, w, cx| root.focus(w, cx)).unwrap();
    for k in ["t", "w", "right"] {
        key(cx, h, k);
        assert!(is_open(cx, &shell), "{k}");
        assert_eq!((state(cx, &shell), view(cx)), before, "{k} did nothing behind the picker");
    }
}

/// Tab and Shift-Tab cycle inside the card: the two year arrows and the keyboard's month.
#[gpui_kit::test]
fn tab_stays_inside_the_card(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-tab", (1280., 900.));
    click(cx, h, "mini-title");
    let card_focused = |cx: &mut TestAppContext| {
        let ids = ["minipick-prev-year", "minipick-next-year", "minipick-month-2026-10"];
        cx.update_window(h, |_, w, _| ids.map(|id| w.find(id).focused() == Some(true))).unwrap()
    };
    assert_eq!(card_focused(cx), [false, false, true]);
    let mut seen = vec![];
    for k in ["tab", "tab", "tab", "tab"] {
        key(cx, h, k);
        seen.push(card_focused(cx));
    }
    assert_eq!(seen, [[true, false, false], [false, true, false], [false, false, true], [true, false, false]]);
    key(cx, h, "shift-tab");
    assert_eq!(card_focused(cx), [false, false, true], "Shift-Tab wraps backwards");
    // Enter on a month still picks, never a control behind the card.
    key(cx, h, "tab");
    key(cx, h, "enter"); // previous year
    assert!(is_open(cx, &shell));
    assert_eq!(state(cx, &shell).1, "October 2026");
}

/// The arrow keeps the focus, so Enter steps several years.
#[gpui_kit::test]
fn a_year_arrow_keeps_focus_so_enter_steps_years(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-arrow", (1280., 900.));
    click(cx, h, "mini-title");
    key(cx, h, "tab");
    key(cx, h, "tab"); // next-year
    for _ in 0..3 {
        key(cx, h, "enter");
    }
    assert!(exists(cx, h, "minipick-month-2029-10"));
    click(cx, h, "minipick-next-year");
    assert!(cx.update_window(h, |_, w, _| w.find("minipick-next-year").focused() == Some(true)).unwrap());
    assert!(is_open(cx, &shell));
}

/// Mini month and main view in different years: the picked month moves both.
#[gpui_kit::test]
fn a_month_picked_in_another_year_moves_the_mini_month_and_the_view(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-years", (1280., 900.));
    click(cx, h, "mini-next"); // the mini month alone moves on
    click(cx, h, "mini-next");
    click(cx, h, "mini-next");
    assert_eq!(state(cx, &shell), (day(2026, 10, 28), "January 2027".into()));
    click(cx, h, "mini-title");
    assert_eq!(cx.update(|cx| shell.read(cx).minipick_cursor()), (2027, 1), "it opens on the mini month's year");
    click(cx, h, "minipick-prev-year");
    click(cx, h, "minipick-prev-year");
    click(cx, h, "minipick-month-2025-06");
    assert_eq!(state(cx, &shell), (day(2025, 6, 28), "June 2025".into()));
    cx.update_window(h, |_, w, _| assert!(w.try_find("month-cell-2025-06-28".to_string()).is_some())).unwrap();
}

#[gpui_kit::test]
fn home_and_end_go_to_the_ends_of_the_year_and_modified_arrows_are_ignored(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-homeend", (1280., 900.));
    click(cx, h, "mini-title");
    let cursor = |cx: &mut TestAppContext| cx.update(|cx| shell.read(cx).minipick_cursor());
    key(cx, h, "end");
    assert_eq!(cursor(cx), (2026, 12));
    key(cx, h, "home");
    assert_eq!(cursor(cx), (2026, 1));
    for k in ["cmd-right", "shift-right", "alt-down", "ctrl-pagedown", "cmd-end"] {
        key(cx, h, k);
        assert_eq!(cursor(cx), (2026, 1), "{k} is not a plain key");
    }
}

/// The card is under the title when there is room, above it when there is not, and never outside the window.
#[gpui_kit::test]
fn the_card_is_not_clipped_by_the_sidebar_or_the_window(cx: &mut TestAppContext) {
    for (w, h_) in [(1120., 600.), (1120., 720.), (1280., 900.), (1440., 640.)] {
        let (h, _) = open(cx, &format!("mp-fit-{w}-{h_}"), (w, h_));
        click(cx, h, "mini-title");
        cx.update_window(h, |_, win, _| {
            let (card, title) = (win.find("minipick").bounds(), win.find("mini-title").bounds());
            let inside = card.left() >= px(8.) && card.top() >= px(8.) && card.right() <= px(w - 8.) && card.bottom() <= px(h_ - 8.);
            assert!(inside, "{w}x{h_}: card {card:?} leaves the window");
            assert!(card.bottom() <= title.top() || card.top() >= title.bottom(), "{w}x{h_}: card {card:?} covers the title {title:?}");
            assert!((card.center().x - title.center().x).abs() <= px(1.), "{w}x{h_}: card is centred on the title");
            for m in 1..=12 {
                let cell = win.find(format!("minipick-month-2026-{m:02}")).bounds();
                assert!(card.contains(&cell.center()), "{w}x{h_}: month {m} is inside the card");
            }
        })
        .unwrap();
    }
}

/// Below 1120 the sidebar is a rail with no mini month, so the picker has no title to open from.
#[gpui_kit::test]
fn the_rail_has_no_picker(cx: &mut TestAppContext) {
    for w in [960., 1119.] {
        let (h, shell) = open(cx, &format!("mp-rail-{w}"), (w, 600.));
        assert!(!exists(cx, h, "mini-title") && !exists(cx, h, "minipick"), "{w}");
        assert!(!is_open(cx, &shell));
    }
}

/// Resizing to the rail while it is open closes it: nothing is left floating without its title.
#[gpui_kit::test]
fn the_picker_closes_when_the_sidebar_collapses(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-shrink", (1280., 800.));
    click(cx, h, "mini-title");
    cx.simulate_window_resize(h, gpui_kit::size(px(1000.), px(800.)));
    frames(cx, h);
    assert!(!is_open(cx, &shell) && !exists(cx, h, "minipick"));
}

fn cursor(cx: &mut TestAppContext, shell: &Entity<Shell>) -> (i32, u32) {
    cx.update(|cx| shell.read(cx).minipick_cursor())
}

fn focused(cx: &mut TestAppContext, h: AnyWindowHandle, id: &str) -> bool {
    cx.update_window(h, |_, w, _| w.find(id.to_string()).focused() == Some(true)).unwrap()
}

/// On a year arrow the grid keys are not the card's: the cursor month stays and the focus stays on the arrow.
#[gpui_kit::test]
fn grid_keys_on_a_year_arrow_change_nothing(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-arrow-keys", (1280., 900.));
    click(cx, h, "mini-title");
    key(cx, h, "tab");
    key(cx, h, "tab"); // next-year
    for k in ["left", "right", "up", "down", "home", "end", "pageup", "pagedown"] {
        key(cx, h, k);
        assert_eq!(cursor(cx, &shell), (2026, 10), "{k} moved the cursor month");
        assert!(focused(cx, h, "minipick-next-year"), "{k} stole the focus from the arrow");
    }
}

/// The year is held to 1..=9999 at both ends, by arrow clicks, arrow keys and Page Up / Page Down; nothing panics.
#[gpui_kit::test]
fn the_year_stops_at_both_ends(cx: &mut TestAppContext) {
    for (name, year, month, step, edge) in [("hi", 9999, 12, "next", 9999), ("lo", 1, 1, "prev", 1)] {
        let (h, shell) = open(cx, &format!("mp-clamp-{name}"), (1280., 900.));
        cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| { s.model.pick_month(year, month); s.sync(cx); let _ = w; })).unwrap();
        frames(cx, h);
        click(cx, h, "mini-title");
        click(cx, h, &format!("minipick-{step}-year"));
        assert_eq!(cursor(cx, &shell).0, edge, "{name}: arrow");
        click(cx, h, &format!("minipick-month-{edge}-{month:02}"));
        assert_eq!(chrono::Datelike::year(&state(cx, &shell).0), edge, "{name}: picked");
        click(cx, h, "mini-title");
        let (page, arrow) = if name == "hi" { ("pagedown", "right") } else { ("pageup", "left") };
        for k in [page, arrow, "end", "home"] {
            key(cx, h, k);
        }
        assert_eq!(cursor(cx, &shell).0, edge, "{name}: keys");
        assert!(is_open(cx, &shell));
    }
}

/// Appearance has no calendar behind it, and a picked month is a picked day's twin: it leaves Appearance for the view, moved.
#[gpui_kit::test]
fn picking_a_month_on_appearance_leaves_it(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-appearance", (1280., 900.));
    click(cx, h, "account-menu");
    assert_eq!(cx.update(|cx| shell.read(cx).model.screen), gpui_calendar::ui::model::Screen::Appearance);
    click(cx, h, "mini-title");
    assert!(is_open(cx, &shell));
    click(cx, h, "minipick-month-2026-03");
    assert_eq!(cx.update(|cx| shell.read(cx).model.screen), gpui_calendar::ui::model::Screen::Calendar);
    assert_eq!(state(cx, &shell), (day(2026, 3, 28), "March 2026".into()));
}

/// An open event card owns the window: the title does not open the picker under or over it (its backdrop, below the picker's in the paint order, would not catch the click).
#[gpui_kit::test]
fn the_picker_does_not_open_over_the_event_card(cx: &mut TestAppContext) {
    use gpui_calendar::{domain::event::Event, store::WORK};
    let soon = day(2026, 10, 28).and_hms_opt(19, 0, 0).unwrap();
    let (h, shell) = open_sized(cx, &tmp("mp-event"), (1280., 900.), Rc::new(Cell::new(at(28, 17, 35))), move |s| {
        s.model.store.events = vec![Event::timed(1, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1))];
    });
    frames(cx, h);
    cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx))).unwrap();
    frames(cx, h);
    assert!(exists(cx, h, "pop-backdrop"));
    // The keyboard reaches the title without a click, so the backdrop does not stand in the way.
    let focus = cx.update(|cx| shell.read(cx).minipick_title_focus());
    cx.update_window(h, |_, w, cx| focus.focus(w, cx)).unwrap();
    key(cx, h, "enter");
    assert!(!is_open(cx, &shell) && !exists(cx, h, "minipick"), "Enter on the title opened the picker over the event card");
    click(cx, h, "mini-title");
    assert!(!is_open(cx, &shell) && !exists(cx, h, "minipick"));
}

/// Collapsing to the rail while the picker is open closes it through the usual path: focus does not stay on a month cell that is gone.
#[gpui_kit::test]
fn collapsing_to_the_rail_returns_focus_to_the_root(cx: &mut TestAppContext) {
    let (h, shell) = open(cx, "mp-shrink-focus", (1280., 800.));
    click(cx, h, "mini-title");
    assert!(focused(cx, h, "minipick-month-2026-10"));
    cx.simulate_window_resize(h, gpui_kit::size(px(1000.), px(800.)));
    frames(cx, h);
    assert!(!is_open(cx, &shell));
    assert!(cx.update_window(h, |_, w, cx| shell.read(cx).minipick_root_focused(w, cx)).unwrap(), "focus fell back to the root");
}
