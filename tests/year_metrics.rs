//! The Year view's metrics from mock v8: grid padding 24, column gap 24, row gap 16, dot gap 2, and twelve months fit at 1440x900.
mod common;

use common::year::*;
use gpui_calendar::domain::year::{DOT_GAP, DOT_PX, ROW_GAP};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext, px};

#[gpui_kit::test]
fn padding_column_gap_and_dot_gap_follow_the_mock(cx: &mut TestAppContext) {
    // Oct 14 holds events of all three calendars: three dots.
    let (h, _) = open_year_with(cx, "yearm-metrics", (1440., 900.), evs());
    cx.update_window(h, |_, w, _| {
        let (view, jan, feb) = (w.find("year-scroll").bounds(), w.find("year-month-2026-01").bounds(), w.find("year-month-2026-02").bounds());
        assert_eq!(jan.left() - view.left(), px(24.), "grid padding left");
        assert_eq!(jan.top() - view.top(), px(24.), "grid padding top");
        assert_eq!(feb.left() - jan.right(), px(24.), "column gap");
        let (a, b) = (w.find("year-dot-2026-10-14-0").bounds(), w.find("year-dot-2026-10-14-1").bounds());
        assert_eq!(b.left() - a.right(), px(DOT_GAP), "dot gap");
        assert_eq!(a.size.width, px(DOT_PX));
        let (last, view) = (w.find("year-pad-12-2027-01-10").bounds(), w.find("year-scroll").bounds());
        assert!(last.bottom() <= view.bottom(), "twelve months fit without scrolling: {last:?} in {view:?}");
    })
    .unwrap();
}

#[gpui_kit::test]
fn month_rows_are_16_apart_and_the_grid_ends_in_24(cx: &mut TestAppContext) {
    // Too short for twelve months: scrolled to the end, the last row meets the bottom padding.
    let (h, _) = open_year_with(cx, "yearm-rows", (1440., 600.), evs());
    cx.update_window(h, |_, w, cx| {
        let (may, last_of_jan) = (w.find("year-month-2026-05").bounds(), w.find("year-pad-01-2026-02-08").bounds());
        assert_eq!(may.top() - last_of_jan.bottom(), px(ROW_GAP), "gap between month rows");
        w.scroll("year-scroll", gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-5000.))), cx);
        w.render_frame(cx);
        let (last, view) = (w.find("year-pad-12-2027-01-10").bounds(), w.find("year-scroll").bounds());
        assert_eq!(view.bottom() - last.bottom(), px(24.), "bottom padding");
    })
    .unwrap();
}
