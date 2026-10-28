//! Header controls (view switch, previous, Today, next): rest, hover and pressed pixels equal the state table, light and dark. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::ui::model::View;
    use headless::states::*;

    run(
        "header",
        vec![Group::new(View::Month, vec![], |_| {
            vec![
                // Segments sit on the switch's `field` fill; the open view is `page`.
                Spec::ghost("view-day", |t| t.neutral.field, |t| t.neutral.field),
                Spec::ghost("view-week", |t| t.neutral.field, |t| t.neutral.field),
                Spec::ghost("view-year", |t| t.neutral.field, |t| t.neutral.field),
                Spec::selected("view-month", |t| t.neutral.page, |t| t.neutral.field),
                // The pills are `page` on the card's `surface`.
                Spec::ghost("nav-prev", |t| t.neutral.page, |t| t.neutral.surface),
                Spec::ghost("nav-today", |t| t.neutral.page, |t| t.neutral.surface),
                Spec::ghost("nav-next", |t| t.neutral.page, |t| t.neutral.surface),
            ]
        })],
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
