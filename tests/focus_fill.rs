//! A focused element with no background of its own must not show the ring colour through its interior:
//! gpui paints a box-shadow behind the element without clipping it, so the gap/ring pair needs a fill.
//! Headless screenshots in both modes; harness-less like `screenshot.rs` (GPUI's offscreen renderer is Metal).
#[cfg(target_os = "macos")]
fn main() {
    use gpui_calendar::store::settings::Mode;
    for mode in [Mode::Dark, Mode::Light] {
        m::month(mode);
        m::year(mode);
        m::controls(mode);
        m::popover(mode);
        m::appearance(mode);
        m::unfocused(mode);
    }
    println!("focus fill ok: {} checks", m::CHECKS.load(std::sync::atomic::Ordering::Relaxed));
}
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod m {
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use chrono::NaiveDate;
    use gpui_calendar::domain::event::Event;
    use gpui_calendar::store::settings::Mode;
    use gpui_calendar::store::{CALENDAR_NAMES, Store, WORK};
    use gpui_calendar::theme::palette::{ACCENTS, NEUTRALS, SLOTS};
    use gpui_calendar::theme::contrast::{Rgb, contrast};
    use gpui_calendar::theme::resolve::{Theme, resolve};
    use gpui_calendar::ui::Shell;
    use gpui_calendar::ui::model::View;
    use gpui_kit::test::{TestEventExt, TestQueryExt, TestWindowExt};
    use gpui_kit::{AnyWindowHandle, AppContext, Bounds, Entity, HeadlessAppContext, MouseButton, WindowBounds, WindowOptions, point, px, size};

    type Cx = HeadlessAppContext;
    /// Every assertion that ran, so the final line reports what was checked, not what was meant to be.
    pub static CHECKS: AtomicUsize = AtomicUsize::new(0);
    fn check(ok: bool, msg: impl FnOnce() -> String) {
        assert!(ok, "{}", msg());
        CHECKS.fetch_add(1, Ordering::Relaxed);
    }
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

    /// The pixels inside `b`, `inset` px in from its edge.
    fn region(cx: &mut Cx, h: AnyWindowHandle, b: Bounds<gpui_kit::Pixels>, inset: f32) -> Vec<Rgb> {
        let img = cx.capture_screenshot(h).expect("offscreen rendering");
        let k = img.width() as f32 / W;
        let f = |p: gpui_kit::Pixels, d: f32| ((f32::from(p) + d) * k) as u32;
        let (x0, x1, y0, y1) = (f(b.left(), inset), f(b.right(), -inset), f(b.top(), inset), f(b.bottom(), -inset));
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).map(|(x, y)| img.get_pixel(x, y)).map(|p| Rgb(p[0], p[1], p[2])).collect()
    }

    fn near(a: Rgb, r: Rgb) -> bool {
        a.0.abs_diff(r.0).max(a.1.abs_diff(r.1)).max(a.2.abs_diff(r.2)) <= 6
    }

    /// The most common colour: what the element is painted with.
    fn interior(all: &[Rgb]) -> Rgb {
        *all.iter().max_by_key(|&&a| all.iter().filter(|&&p| p == a).count()).unwrap()
    }

    /// The element's interior must not be the ring colour, and its text must read against what is there.
    fn assert_filled(cx: &mut Cx, h: AnyWindowHandle, what: &str, mode: Mode, ring: Rgb, b: Bounds<gpui_kit::Pixels>) {
        filled(cx, h, what, mode, ring, b, 3.);
    }

    /// `min_text` is 1 for a swatch: its dot is a calendar colour, not text.
    fn filled(cx: &mut Cx, h: AnyWindowHandle, what: &str, mode: Mode, ring: Rgb, b: Bounds<gpui_kit::Pixels>, min_text: f64) {
        let all = region(cx, h, b, 3.);
        assert!(!all.is_empty(), "{what} {mode:?}: no pixels in {b:?}");
        let share = all.iter().filter(|&&p| near(p, ring)).count() as f32 / all.len() as f32;
        let inner = interior(&all);
        let text = all.iter().map(|&p| contrast(p, inner)).fold(1., f64::max);
        check(share < 0.5 && !near(inner, ring) && text >= min_text, || format!("{what} {mode:?}: ring-colour share {share:.2}, interior {inner:?}, best text contrast {text:.1}"));
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
    pub fn month(mode: Mode) {
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
    }

    /// The year grid's cursor day and month title carry their ring on an overlay that sits above the content.
    pub fn year(mode: Mode) {
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
    }

    /// The focused control among `ids`, with its bounds.
    fn focused(cx: &mut Cx, h: AnyWindowHandle, ids: &[String]) -> Option<(String, Bounds<gpui_kit::Pixels>)> {
        cx.update_window(h, |_, w, _| ids.iter().find(|id| w.try_find(id.to_string()).is_some_and(|e| e.focused() == Some(true))).map(|id| (id.clone(), w.find(id.to_string()).bounds()))).unwrap()
    }

    /// What a sweep presses and which controls it must reach; `ring_of` gives the ring colour of an id.
    struct Sweep<'a> {
        ids: &'a [String],
        key: &'a str,
        tabs: usize,
        arrows: usize,
        ring_of: &'a dyn Fn(&str) -> Rgb,
        min_text: &'a dyn Fn(&str) -> f64,
    }

    /// Tab `tabs` times, check every focused control among `ids`, and require that each one was reached.
    /// `ring_of` is the ring colour for an id; `arrows` presses Right on a roving group member to reach its siblings.
    fn sweep(cx: &mut Cx, h: AnyWindowHandle, mode: Mode, spec: Sweep) {
        let Sweep { ids, key, tabs, arrows, ring_of, min_text } = spec;
        let mut seen = HashSet::new();
        let visit = |cx: &mut Cx, seen: &mut HashSet<String>| -> Option<String> {
            let (id, b) = focused(cx, h, ids)?;
            filled(cx, h, &id, mode, ring_of(&id), b, min_text(&id));
            seen.insert(id.clone());
            Some(id)
        };
        for _ in 0..tabs {
            act(cx, h, |w, cx| w.key_down(key, false, cx));
            if visit(cx, &mut seen).is_some_and(|id| id.starts_with("pop-cal-")) {
                for _ in 0..arrows {
                    act(cx, h, |w, cx| w.key_down("right", false, cx));
                    visit(cx, &mut seen);
                }
            }
            if seen.len() == ids.len() {
                break;
            }
        }
        let want: HashSet<String> = ids.iter().cloned().collect();
        let mut missing: Vec<_> = want.difference(&seen).collect();
        missing.sort();
        check(seen == want, || format!("{mode:?}: tab never focused {missing:?}"));
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    /// Tab to each bg-less control and check its interior. Controls that paint their own fill are in the list too:
    /// they must stay filled.
    pub fn controls(mode: Mode) {
        // add-event is ink-filled (ink is the ring colour), so it is not in the list. open-event rings white on the banner.
        let ids = strings(&["nav-prev", "nav-next", "nav-today", "view-month", "view-week", "view-day", "view-year", "calendar-row-0", "calendar-row-2", "mini-prev", "mini-next", "mini-day-2026-10-14", "account-menu", "open-event"]);
        let (mut cx, h, _, t) = open(mode, View::Month, vec![soon_event()]);
        sweep(&mut cx, h, mode, Sweep { ids: &ids, key: "tab", tabs: 120, arrows: 0, ring_of: &|id| if id == "open-event" { Rgb::WHITE } else { t.ring }, min_text: &|_| 3. });
    }

    fn soon_event() -> Event {
        let soon = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap().and_hms_opt(19, 0, 0).unwrap();
        Event::timed(1, "Piano lesson", WORK, soon, soon + chrono::Duration::hours(1))
    }

    /// The event card of an existing event. The notes box swallows Tab (and Shift+Tab), so Delete and Cancel are
    /// reached backwards from the end of the trap on a fresh card, and the rest forwards on another. Save is ink-filled.
    pub fn popover(mode: Mode) {
        let card = || {
            let (mut cx, h, shell, t) = open(mode, View::Month, vec![soon_event()]);
            cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx))).unwrap();
            render(&mut cx, h);
            (cx, h, t)
        };
        let (mut cx, h, t) = card();
        sweep(&mut cx, h, mode, Sweep { ids: &strings(&["pop-delete", "pop-cancel"]), key: "shift-tab", tabs: 4, arrows: 0, ring_of: &|_| t.ring, min_text: &|_| 3. });
        let (mut cx, h, t) = card();
        let mut ids: Vec<String> = (0..CALENDAR_NAMES.len()).map(|i| format!("pop-cal-{i}")).collect();
        ids.push("pop-all-day".into());
        sweep(&mut cx, h, mode, Sweep { ids: &ids, key: "tab", tabs: 3, arrows: CALENDAR_NAMES.len(), ring_of: &|_| t.ring, min_text: &|_| 3. });
        // The undo toast sits on an ink bar and rings in on-ink.
        cx.update_window(h, |_, w, cx| w.click("pop-delete", cx)).unwrap();
        render(&mut cx, h);
        sweep(&mut cx, h, mode, Sweep { ids: &strings(&["toast-undo"]), key: "tab", tabs: 120, arrows: 0, ring_of: &|_| t.on_ink, min_text: &|_| 3. });
    }

    /// The Appearance screen: modes, accents, tints, every calendar swatch and the reset link.
    pub fn appearance(mode: Mode) {
        let (mut cx, h, _, t) = open(mode, View::Month, vec![]);
        act(&mut cx, h, |w, cx| w.click("account-menu", cx));
        // The chosen mode pill is ink-filled, and ink is the ring colour, like add-event: `unfocused` covers it.
        let chosen = if mode == Mode::Dark { "mode-dark" } else { "mode-light" };
        let mut ids: Vec<String> = ["mode-light", "mode-dark", "mode-system", "reset-appearance"].iter().filter(|&&id| id != chosen).map(|s| s.to_string()).collect();
        ids.extend(ACCENTS.iter().map(|a| format!("accent-{a:?}")));
        ids.extend(NEUTRALS.iter().map(|n| format!("neutral-{n:?}")));
        ids.extend((0..CALENDAR_NAMES.len()).flat_map(|i| SLOTS.iter().map(move |c| format!("calendar-color-{i}-{c:?}"))));
        // A swatch is a calendar colour, not text.
        sweep(&mut cx, h, mode, Sweep { ids: &ids, key: "tab", tabs: 200, arrows: 0, ring_of: &|_| t.ring, min_text: &|id| if id.starts_with("accent-") || id.starts_with("calendar-color-") { 1. } else { 3. } });
    }

    /// An unfocused control paints exactly what it did before the ring existed: its own fill, or what is behind it.
    pub fn unfocused(mode: Mode) {
        let (mut cx, h, shell, t) = open(mode, View::Month, vec![soon_event()]);
        cx.update_window(h, |_, w, cx| shell.update(cx, |s, cx| s.show_event(1, w, cx))).unwrap();
        render(&mut cx, h);
        let paint = |cx: &mut Cx, id: &str| {
            let b = cx.update_window(h, |_, w, _| w.find(id.to_string()).bounds()).unwrap();
            interior(&region(cx, h, b, 3.))
        };
        let (cancel, save) = (paint(&mut cx, "pop-cancel"), paint(&mut cx, "pop-save"));
        check(near(cancel, t.neutral.surface), || format!("{mode:?}: Cancel kept {cancel:?}, wanted surface {:?}", t.neutral.surface));
        check(near(save, t.ink), || format!("{mode:?}: Save kept {save:?}, wanted ink {:?}", t.ink));
        // Delete has no fill: its interior is the footer behind it, whatever its `gap` argument is.
        let b = cx.update_window(h, |_, w, _| w.find("pop-delete").bounds()).unwrap();
        let (delete, behind) = (interior(&region(&mut cx, h, b, 3.)), interior(&region(&mut cx, h, Bounds { origin: point(b.left() - px(12.), b.top()), size: size(px(8.), b.size.height) }, 0.)));
        check(near(delete, behind), || format!("{mode:?}: Delete paints {delete:?} over a footer of {behind:?}"));
        // The chosen mode pill keeps the ink fill it was given.
        act(&mut cx, h, |w, cx| w.click("pop-cancel", cx));
        act(&mut cx, h, |w, cx| w.click("account-menu", cx));
        let on = paint(&mut cx, if mode == Mode::Dark { "mode-dark" } else { "mode-light" });
        check(near(on, t.ink), || format!("{mode:?}: the chosen mode pill kept {on:?}, wanted ink {:?}", t.ink));
    }
}
