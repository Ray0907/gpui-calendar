//! Sidebar controls (add, calendar rows, mini month, account menu, banner button): rest, hover and pressed pixels equal the state table. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::ui::model::View;
    use headless::states::*;

    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::WORK;
    let soon = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(19, 0, 0).unwrap();
    let events = vec![Event::timed(1, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1))];
    run(
        "sidebar",
        vec![Group::new(View::Month, events, |_| {
            vec![
                Spec::filled("add-event"),
                Spec::row("calendar-row-0", |t| t.neutral.surface),
                Spec::row("calendar-row-2", |t| t.neutral.surface),
                Spec::ghost("mini-prev", |t| t.neutral.surface, |t| t.neutral.surface),
                Spec::ghost("mini-title", |t| t.neutral.surface, |t| t.neutral.surface),
                Spec::ghost("mini-next", |t| t.neutral.surface, |t| t.neutral.surface),
                Spec::ghost("mini-day-2026-10-14", |t| t.neutral.surface, |t| t.neutral.surface),
                Spec::ghost("account-menu", |t| t.neutral.surface, |t| t.neutral.surface),
                Spec::banner("open-event"),
            ]
        })],
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
