//! Mock v8 sidebar mini calendar, as offsets from the mini month's top (the card's position depends on the window, not the mini).
//! The mini is 6 + 28 title + 6 + 24 weekdays + 2 + (rows x 28 + (rows - 1) x 2) + 6; in the mock the weekday row is a row of the day grid,
//! so the 2px row gap sits under it too. The Year grid's top padding is checked in `year_metrics`.
mod common;

use chrono::{Datelike, NaiveDate};
use common::year::*;
use gpui_calendar::ui::Shell;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, px};

/// Layout lands on fractional pixels; positions agree with the mock to within a tenth of one.
fn close(got: impl Into<f32>, want: f32, what: &str) {
    let got = got.into();
    assert!((got - want).abs() < 0.1, "{what}: {got} vs {want}");
}

/// The mini month's height for `rows` week rows: padding, title, gap, weekdays, gap, rows with their gaps, padding.
fn mini_height(rows: u32) -> f32 {
    6. + 28. + 6. + 24. + 2. + (rows as f32 * 28. + (rows - 1) as f32 * 2.) + 6.
}

/// Monday-first rows of `month` (the default week start), and the first cell's date.
fn grid_of(month: NaiveDate) -> (NaiveDate, u32) {
    let lead = month.weekday().num_days_from_monday();
    let next = if month.month() == 12 { d(month.year() + 1, 1, 1) } else { d(month.year(), month.month() + 1, 1) };
    (month - chrono::Duration::days(lead as i64), (lead + (next - month).num_days() as u32).div_ceil(7))
}

#[gpui_kit::test]
fn mini_calendar_rows_follow_the_mock_relative_to_its_top(cx: &mut gpui_kit::TestAppContext) {
    let (h, _) = open_year_with(cx, "mock-metrics", (1440., 900.), evs());
    cx.update_window(h, |_, w, _| {
        let top = f32::from(w.find("mini-month").bounds().top());
        let (first, rows) = grid_of(d(2026, 10, 1));
        assert_eq!(rows, 5, "October 2026 spans five weeks");
        let mid = f32::from(w.find("mini-title").bounds().center().y);
        assert!((mid - (top + 6. + 14.)).abs() < 0.5, "mini title row centre {mid}");
        close(f32::from(w.find("mini-weekday-0").bounds().top()), top + 6. + 28. + 6., "mini weekday row top");
        for r in 0..rows {
            let date = first + chrono::Duration::days(7 * r as i64);
            let want = top + 6. + 28. + 6. + 24. + 2. + r as f32 * 30.;
            close(f32::from(w.find(format!("mini-day-{date}")).bounds().top()), want, &format!("{date} row top"));
        }
        close(f32::from(w.find("mini-month").bounds().size.height), mini_height(rows), "mini height");
    })
    .unwrap();
}

/// A six-week month (August 2026) in the shortest window with the full sidebar (`RAIL_BELOW` wide, `MIN_WINDOW` tall): the calendar list gives way, and the mini month, the account row and the
/// card edge all stay where they should.
#[gpui_kit::test]
fn a_six_week_mini_month_in_a_short_window_clips_nothing(cx: &mut gpui_kit::TestAppContext) {
    for size in [(1120., 600.), (1440., 900.)] {
        let (h, shell) = open_year_with(cx, "mock-metrics-aug", size, evs());
        cx.update(|cx| shell.update(cx, |s: &mut Shell, _| (0..2).for_each(|_| s.model.mini_step(-1))));
        cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
        assert_eq!(cx.update(|cx| shell.read(cx).model.mini_title().to_string()), "August 2026", "the anchor month is August 2026, not a five-week one");
        cx.update_window(h, |_, w, _| {
            let (list, mini, account, card) = (w.find("calendar-list").bounds(), w.find("mini-month").bounds(), w.find("account-menu").bounds(), w.find("sidebar").bounds());
            close(f32::from(mini.size.height), mini_height(6), &format!("six-week mini keeps its height at {size:?}"));
            assert!(list.size.height >= px(36.), "calendar list keeps its floor at {size:?}: {:?}", list.size.height);
            // The list's clip reaches 4px past its rows (the focus ring's room, a negative margin), so it ends 4px under the mini's top edge at most.
            assert!(list.bottom() - px(4.) <= mini.top(), "list ends above the mini at {size:?}");
            assert!(mini.bottom() <= account.top(), "mini ends above the account row at {size:?}");
            assert!(account.bottom() <= card.bottom(), "account row inside the card at {size:?}");
            let (first, rows) = grid_of(d(2026, 8, 1));
            assert_eq!(rows, 6, "August 2026 spans six weeks");
            let last = first + chrono::Duration::days(7 * 5);
            assert!(w.find(format!("mini-day-{last}")).bounds().bottom() <= mini.bottom(), "last row inside the mini at {size:?}");
        })
        .unwrap();
    }
}

/// The weekday letters stay on one row at window widths that divide the sidebar into fractions, and the first day row sits 24 + 2 below them.
#[gpui_kit::test]
fn the_weekday_row_never_wraps_at_odd_widths(cx: &mut gpui_kit::TestAppContext) {
    for width in [1121., 1133.5, 1297., 1440.25] {
        let (h, _) = open_year_with(cx, "mock-metrics-odd", (width, 900.), evs());
        cx.update_window(h, |_, w, _| {
            let top = w.find("mini-weekday-0").bounds().top();
            for i in 1..7 {
                close(f32::from(w.find(format!("mini-weekday-{i}")).bounds().top()), f32::from(top), &format!("weekday {i} shares the row at {width}"));
            }
            let (first, _) = grid_of(d(2026, 10, 1));
            close(f32::from(w.find(format!("mini-day-{first}")).bounds().top()), f32::from(top) + 24. + 2., &format!("first day row at {width}"));
        })
        .unwrap();
    }
}
