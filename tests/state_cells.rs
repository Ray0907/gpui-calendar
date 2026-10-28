//! Day cells, event chips, blocks, pills and "+n more" in the Month, Year and Week views: rest, hover and pressed pixels equal the state table
//! (`wash-hover` over the cell's own fill; a chip adds it over its tint), light and dark. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::WORK;
    use gpui_calendar::ui::model::View;
    use headless::states::*;

    // Cells, chips and counts hold a roving cursor, not a Tab stop: focus is covered by focus_fill.rs, so these specs skip the Tab check.
    fn nf(v: Vec<Spec>) -> Vec<Spec> {
        v.into_iter().map(Spec::nofocus).collect()
    }
    let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
    let timed = |id, d, h, title: &str| Event::timed(id, title, WORK, day(d).and_hms_opt(h, 0, 0).unwrap(), day(d).and_hms_opt(h + 1, 0, 0).unwrap());
    let tint = |t: &gpui_calendar::theme::resolve::Theme| t.cal(WORK).expect("work theme").tint;

    let month: Vec<Event> = [vec![Event::all_day(10, "Work block", WORK, day(6), day(7)), timed(11, 14, 10, "Standup")], (0..8).map(|i| timed(30 + i, 21, 8 + i as u32, "Busy")).collect()].concat();
    let week = vec![timed(20, 28, 10, "Review"), Event::all_day(21, "Offsite", WORK, day(27), day(28)), timed(22, 29, 9, "A"), timed(23, 29, 9, "B"), timed(24, 29, 9, "C"), timed(25, 29, 9, "D")];
    // Item 4: pressing a chip dims the chip only; the cell around it does not move.
    for mode in [gpui_calendar::store::settings::Mode::Light, gpui_calendar::store::settings::Mode::Dark] {
        let mut win = headless::open(mode, gpui_calendar::store::settings::Neutral::Cool, View::Month, month.clone(), (1440., 900.));
        let moved = leaf_press_moves_container(&mut win, "month-cell-2026-10-14", "month-ev-11-*");
        assert!(moved <= 3, "{mode:?}: pressing a chip moved its cell's date number by {moved}");
    }
    run(
        "cells",
        vec![
            Group::new(View::Month, month, move |_| {
                nf(vec![
                    Spec::cell("month-cell-2026-10-13", |t| t.neutral.surface),
                    // The weekend band is `subtle`: the wash goes over it, never `subtle` itself.
                    Spec::cell("month-cell-2026-10-24", |t| t.neutral.subtle),
                    Spec::chip("month-ev-10-*", tint, |t| t.neutral.surface),
                    Spec::ghost("month-ev-11-*", |t| t.neutral.surface, |t| t.neutral.surface),
                    Spec::ghost("month-more*", |t| t.neutral.surface, |t| t.neutral.surface),
                ])
            }),
            Group::new(View::Year, vec![], |_| {
                nf(vec![
                    Spec::ghost("year-day-2026-10-07", |t| t.neutral.surface, |t| t.neutral.surface),
                    Spec::ghost("year-day-2026-10-10", |t| t.neutral.subtle, |t| t.neutral.subtle),
                    Spec::ghost("year-month-2026-10", |t| t.neutral.surface, |t| t.neutral.surface),
                ])
            }),
            Group::new(View::Week, week, move |_| {
                nf(vec![Spec::chip("week-ev-20-*", tint, |t| t.neutral.surface), Spec::chip("week-ad-21", tint, |t| t.neutral.surface), Spec::ghost("week-more*", |t| t.neutral.surface, |t| t.neutral.surface)])
            }),
        ],
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
