//! A focused element with no background of its own must not show the ring colour through its interior:
//! gpui paints a box-shadow behind the element without clipping it, so the gap/ring pair needs a fill.
//! Headless screenshots in both modes; harness-less like `screenshot.rs` (GPUI's offscreen renderer is Metal).
#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::Mode;
    let mut ran = 0;
    for mode in [Mode::Dark, Mode::Light] {
        ran += m::month(mode) + m::year(mode) + m::controls(mode);
    }
    println!("focus fill ok: {ran} checks");
}
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod m {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::settings::Mode;
    use gpui_calendar::store::{Store, WORK};
    use gpui_calendar::theme::contrast::{Rgb, contrast};
    use gpui_calendar::theme::resolve::{Theme, resolve};
    use gpui_calendar::ui::Shell;
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
    use gpui_kit::{AnyWindowHandle, AppContext, Bounds, Entity, HeadlessAppContext, MouseButton, WindowBounds, WindowOptions, point, px, size};

    type Cx = HeadlessAppContext;
    const W: f32 = 1440.;

    fn open(mode: Mode, view: View, events: Vec<Event>) -> (Cx, AnyWindowHandle, Entity<Shell>, Theme) {
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(gpui_calendar::assets()),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        let mut store = Store::default();
        store.settings.appearance.mode = mode;
        store.events = events;
        let theme = resolve(&store.settings.appearance, mode == Mode::Dark);
        let now = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(17, 35, 0).unwrap();
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds { origin: Default::default(), size: size(px(W), px(900.)) })),
            focus: false,
            show: false,
            ..gpui_calendar::ui::window_options()
        };
        let (h, shell) = cx.update(|cx| gpui_kit::open_window(opts, cx, |w, cx| cx.new(|cx| Shell::new(store, None, Box::new(move || now), w, cx)))).unwrap();
        cx.update(|cx| {
            shell.update(cx, |s, cx| {
                s.model.set_view(view);
                s.sync(cx);
            })
        });
        render(&mut cx, h);
        (cx, h, shell, theme)
    }

    fn render(cx: &mut Cx, h: AnyWindowHandle) {
        for _ in 0..4 {
            cx.update_window(h, |_, w, cx| w.render_frame(cx)).unwrap();
        }
    }

    fn act(cx: &mut Cx, h: AnyWindowHandle, f: impl FnOnce(&mut gpui_kit::Window, &mut gpui_kit::App)) {
        cx.update_window(h, |_, w, cx| f(w, cx)).unwrap();
        render(cx, h);
    }

    /// The element's interior must not be the ring colour, and its text must read against what is there.
    fn assert_filled(cx: &mut Cx, h: AnyWindowHandle, what: &str, mode: Mode, ring: Rgb, b: Bounds<gpui_kit::Pixels>) {
        let img = cx.capture_screenshot(h).expect("offscreen rendering");
        let k = img.width() as f32 / W;
        let (x0, x1, y0, y1) = {
            let f = |p: gpui_kit::Pixels, d: f32| ((f32::from(p) + d) * k) as u32;
            (f(b.left(), 3.), f(b.right(), -3.), f(b.top(), 3.), f(b.bottom(), -3.))
        };
        let px_at = |x, y| {
            let p = img.get_pixel(x, y);
            Rgb(p[0], p[1], p[2])
        };
        let all: Vec<Rgb> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).map(|(x, y)| px_at(x, y)).collect();
        assert!(!all.is_empty(), "{what} {mode:?}: no pixels in {b:?}");
        let near = |a: Rgb, r: Rgb| a.0.abs_diff(r.0).max(a.1.abs_diff(r.1)).max(a.2.abs_diff(r.2)) <= 6;
        let share = all.iter().filter(|&&p| near(p, ring)).count() as f32 / all.len() as f32;
        // The interior is the most common colour; text is whatever stands out from it.
        let interior = *all.iter().max_by_key(|&&a| all.iter().filter(|&&p| p == a).count()).unwrap();
        let text = all.iter().map(|&p| contrast(p, interior)).fold(1., f64::max);
        assert!(share < 0.5 && !near(interior, ring) && text >= 3., "{what} {mode:?}: ring-colour share {share:.2}, interior {interior:?}, best text contrast {text:.1}");
    }

    fn six_busy() -> Vec<Event> {
        (0..6u32)
            .map(|i| {
                let s = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap().and_hms_opt(8 + i, 0, 0).unwrap();
                Event::timed(100 + u64::from(i), &format!("Busy {i}"), WORK, s, s + chrono::Duration::minutes(30))
            })
            .collect()
    }

    /// A focused chip, a focused "+n more" count and the cursor day's number in the month grid.
    pub fn month(mode: Mode) -> usize {
        let (mut cx, h, _, t) = open(mode, View::Month, six_busy());
        act(&mut cx, h, |w, cx| {
            w.click("month-cell-2026-10-06", cx);
            w.press("right", cx);
        });
        let chip = cx.update_window(h, |_, w, _| w.find("month-ev-100-518").bounds()).unwrap();
        assert_filled(&mut cx, h, "month chip", mode, t.ring, chip);
        act(&mut cx, h, |w, cx| (0..3).for_each(|_| w.press("right", cx)));
        let more = cx.update_window(h, |_, w, _| {
            assert_eq!(w.find_by_label("+3 more").selected(), Some(true), "the count holds the ring");
            w.find_by_label("+3 more").bounds()
        });
        assert_filled(&mut cx, h, "month +n more", mode, t.ring, more.unwrap());
        // No roving event: the grid cursor is the day number, a circle that has no fill unless it is today.
        let (mut cx, h, shell, t) = open(mode, View::Month, vec![]);
        cx.update(|cx| shell.update(cx, |s, _| s.model.anchor = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap()));
        act(&mut cx, h, |w, cx| w.click("month-cell-2026-10-06", cx));
        let cell = cx.update_window(h, |_, w, _| {
            assert_eq!(w.find("month-cell-2026-10-06").selected(), Some(true), "the cell holds the cursor");
            w.find("month-cell-2026-10-06").bounds()
        });
        let c = cell.unwrap();
        // The 22px number box: top border 1 + pt 6, right px 4 + 6.
        let num = Bounds { origin: point(c.right() - px(10. + 22.), c.top() + px(7.)), size: size(px(22.), px(22.)) };
        assert_filled(&mut cx, h, "month day number", mode, t.ring, num);
        3
    }

    /// The year grid's cursor day and month title carry their ring on an overlay that sits above the content.
    pub fn year(mode: Mode) -> usize {
        let (mut cx, h, _, t) = open(mode, View::Year, vec![]);
        act(&mut cx, h, |w, cx| {
            let pad = w.find("year-scroll").bounds().origin + point(px(4.), px(4.));
            w.pointer_down(pad, MouseButton::Left, cx);
            w.pointer_up(pad, MouseButton::Left, cx);
        });
        let day = cx.update_window(h, |_, w, _| w.find("year-day-2026-10-28").bounds()).unwrap();
        assert_filled(&mut cx, h, "year cursor day", mode, t.ring, day);
        act(&mut cx, h, |w, cx| ["up", "up", "up", "left", "left", "left", "up"].iter().for_each(|k| w.key_down(k, false, cx)));
        let title = cx.update_window(h, |_, w, _| w.try_find("year-ring-month-2026-10").map(|_| w.find("year-month-2026-10").bounds())).unwrap();
        assert_filled(&mut cx, h, "year month title", mode, t.ring, title.expect("the cursor reached a month title"));
        2
    }

    /// Tab to each bg-less control and check its interior. Controls that paint their own fill are in the list too:
    /// they must stay filled.
    pub fn controls(mode: Mode) -> usize {
        // add-event is ink-filled (ink is the ring colour), so it is not in the list. open-event rings white on the banner.
        let ids = ["nav-prev", "nav-next", "nav-today", "view-month", "view-week", "view-day", "view-year", "calendar-row-0", "calendar-row-2", "mini-prev", "mini-next", "mini-day-2026-10-14", "account-menu", "open-event"];
        let soon = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(19, 0, 0).unwrap();
        let (mut cx, h, _, t) = open(mode, View::Month, vec![Event::timed(1, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1))]);
        let mut seen = 0;
        for _ in 0..120 {
            act(&mut cx, h, |w, cx| w.press("tab", cx));
            let hit = cx.update_window(h, |_, w, _| ids.iter().find(|id| w.try_find(**id).is_some_and(|e| e.focused() == Some(true))).map(|id| (*id, w.find(*id).bounds()))).unwrap();
            if let Some((id, b)) = hit {
                let ring = if id == "open-event" { Rgb::WHITE } else { t.ring };
                assert_filled(&mut cx, h, id, mode, ring, b);
                seen += 1;
            }
        }
        assert!(seen >= ids.len() - 1, "tab reached only {seen} of {} controls", ids.len());
        seen
    }
}
