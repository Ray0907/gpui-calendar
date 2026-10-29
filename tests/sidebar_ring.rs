//! A calendar row in the sidebar shows its whole focus ring (2px, offset 2px) on all four sides, not clipped by the list that scrolls it.
//! Light and dark, every row: Tab to it, then the ring colour sits 3px out from each edge. Harness-less (Metal).
#[cfg(target_os = "macos")]
mod headless;

#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::{Mode, Neutral};
    use gpui_calendar::ui::model::View;
    use headless::states::{locate, tab_to};
    use headless::*;

    let mut checks = 0;
    for mode in [Mode::Light, Mode::Dark] {
        for row in 0..3 {
            let id = format!("calendar-row-{row}");
            let mut win = open(mode, Neutral::Cool, View::Month, vec![], (1440., 900.));
            assert!(tab_to(&mut win, &id, "tab"), "{mode:?} {id}: not reached by Tab");
            let b = locate(&mut win, &id).expect("row painted");
            let (l, r, t, bt) = (f32::from(b.left()), f32::from(b.right()), f32::from(b.top()), f32::from(b.bottom()));
            let (cx, cy) = (f32::from(b.center().x), f32::from(b.center().y));
            let shot = win.shot();
            for (side, x, y) in [("left", l - 3., cy), ("right", r + 3., cy), ("top", cx, t - 3.), ("bottom", cx, bt + 3.)] {
                let got = shot.mean(x, y, 0.4);
                assert!(delta(got, win.theme.ring) <= 6, "{mode:?} {id}: ring {side} is {got:?}, want {:?}", win.theme.ring);
                checks += 1;
            }
        }
    }
    println!("sidebar ring ok: {checks} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}
