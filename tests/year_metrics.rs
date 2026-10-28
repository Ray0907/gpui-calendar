//! The Year view's metrics from mock v8: grid padding 24, column gap 24, dot gap 2, and twelve months fit at 1440x900.
mod common;

use common::year::*;
use gpui_calendar::domain::year::{DOT_GAP, DOT_PX};
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
