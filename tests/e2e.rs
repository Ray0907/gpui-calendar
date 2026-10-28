//! Persistence E2E: create, move, resize, recolor, quit, relaunch from the same file in a temp dir.
mod common;

use common::{at, tmp};
use gpui_calendar::store::settings::{Accent, CalendarColor, Mode, Neutral, TimeFormat};
use gpui_calendar::store::{Store, WORK};
use gpui_calendar::ui::Shell;
use gpui_calendar::ui::model::View;
use gpui_kit::test::{TestEventExt, TestWindowExt};
use gpui_kit::{AnyWindowHandle, AppContext, Bounds, Entity, MouseButton, Pixels, Point, TestAppContext, Window, point, px};

fn at_min(w: &Window, day: u32, minute: f32) -> Point<Pixels> {
    let b: Bounds<Pixels> = w.find(format!("week-col-2026-10-{day:02}")).bounds();
    point(b.center().x, b.top() + px(minute * 44. / 60.))
}

fn drag(cx: &mut TestAppContext, h: AnyWindowHandle, f: impl Fn(&Window) -> Vec<Point<Pixels>>) {
    cx.update_window(h, |_, w, cx| {
        let path = f(w);
        w.pointer_down(path[0], MouseButton::Left, cx);
        for p in &path[1..] {
            w.pointer_move(*p, Some(MouseButton::Left), cx);
        }
        w.pointer_up(*path.last().unwrap(), MouseButton::Left, cx);
        w.render_frame(cx);
    })
    .unwrap();
}

fn store(cx: &mut TestAppContext, shell: &Entity<Shell>) -> Store {
    cx.update(|cx| shell.read(cx).model.store.clone())
}

/// The shell started on `path` the way the app does, in the Week view of an empty calendar.
fn launch(cx: &mut TestAppContext, path: &std::path::Path, clear: bool) -> (AnyWindowHandle, Entity<Shell>) {
    cx.update(gpui_kit::init);
    let path = path.to_path_buf();
    let now = at(28, 17, 35);
    let mut opts = gpui_calendar::ui::window_options();
    opts.window_bounds = Some(gpui_kit::WindowBounds::Windowed(Bounds { origin: Default::default(), size: gpui_kit::size(px(1440.), px(900.)) }));
    let (h, shell) = cx.update(|cx| gpui_kit::open_window(opts, cx, |window, cx| cx.new(|cx| Shell::start(Some(path), Box::new(move || now), window, cx))).unwrap());
    cx.update(|cx| {
        shell.update(cx, |s, cx| {
            if clear {
                s.model.store.events.clear();
            }
            s.model.set_view(View::Week);
            s.sync(cx);
        })
    });
    cx.update_window(h, |_, w, cx| (0..3).for_each(|_| w.render_frame(cx))).unwrap();
    (h, shell)
}

#[gpui_kit::test]
fn e2e_state_is_identical_after_quit_and_relaunch(cx: &mut TestAppContext) {
    let path = tmp("e2e-relaunch");
    let (h, shell) = launch(cx, &path, true);

    // Create 10:00 to 11:00 on Thursday the 29th, move it a day later and 2 h later, then stretch its end to 14:00.
    drag(cx, h, |w| vec![at_min(w, 29, 10. * 60. + 0.5), at_min(w, 29, 11. * 60. - 0.5)]);
    let made = store(cx, &shell).events;
    assert_eq!(made.len(), 1, "one drag made one event");
    assert_eq!((made[0].start, made[0].end), (at(29, 10, 0), at(29, 11, 0)));
    drag(cx, h, |w| vec![at_min(w, 29, 10. * 60. + 20.5), at_min(w, 30, 12. * 60. + 20.5)]);
    let e = store(cx, &shell).events[0].clone();
    assert_eq!((e.start, e.end), (at(30, 12, 0), at(30, 13, 0)), "moved");
    drag(cx, h, |w| vec![at_min(w, 30, 13. * 60. - 0.7), at_min(w, 30, 14. * 60. - 0.5)]);
    let e = store(cx, &shell).events[0].clone();
    assert_eq!((e.start, e.end), (at(30, 12, 0), at(30, 14, 0)), "resized");

    // Colors and settings. Week start and time format have no control yet, so they are set in the model;
    // the next save (a calendar toggle) carries them.
    cx.update(|cx| {
        shell.update(cx, |s, _| {
            s.model.store.settings.week_start = chrono::Weekday::Sun;
            s.model.store.settings.time_format = TimeFormat::H12;
        })
    });
    cx.update_window(h, |_, w, cx| {
        for id in ["account-menu", "mode-dark", "neutral-Warm", "accent-Violet", "calendar-color-0-Pink", "account-menu", "calendar-row-1"] {
            w.click(id, cx);
            w.render_frame(cx);
        }
    })
    .unwrap();
    let before = store(cx, &shell);
    let a = before.settings.appearance;
    assert_eq!((a.mode, a.neutral, a.accent), (Mode::Dark, Neutral::Warm, Accent::Violet));
    assert_eq!(a.calendar_colors.work, CalendarColor::Pink);
    assert!(!before.calendars[1].visible, "Personal hidden");
    assert_eq!(before.settings.week_start, chrono::Weekday::Sun);
    assert_eq!(before.events.len(), 1);

    // Quit: the window goes away. The file alone carries the state.
    cx.update_window(h, |_, w, _| w.remove_window()).unwrap();
    assert_eq!(Store::load(&path).unwrap(), before, "the file holds exactly what was on screen");

    let (h2, again) = launch(cx, &path, false);
    cx.update_window(h2, |_, w, _| assert!(w.try_find("notice").is_none(), "a clean relaunch says nothing")).unwrap();
    let after = store(cx, &again);
    assert_eq!(after, before, "events, calendars, settings and appearance are identical after relaunch");
    assert_eq!(after.events[0].calendar, WORK);
}

#[gpui_kit::test]
fn e2e_a_corrupt_file_is_kept_and_the_message_is_shown(cx: &mut TestAppContext) {
    let path = tmp("e2e-corrupt");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let junk = b"{ not json at all";
    std::fs::write(&path, junk).unwrap();
    let (h, shell) = launch(cx, &path, false);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        let msg = w.find("notice").label().unwrap().to_string();
        assert!(msg.contains("could not be used") && msg.contains(".bak"), "{msg}");
    })
    .unwrap();
    assert_eq!(std::fs::read(path.with_file_name("calendar.json.bak")).unwrap(), junk, "the damaged bytes are kept");
    assert_eq!(store(cx, &shell), Store::default(), "defaults on screen");
}
