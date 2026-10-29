//! Mock v8 Appearance rhythm, as offsets between controls (the panel's position depends on the window): rows pad 14 above and below, the
//! calendar sub-rows sit 36 apart, and Reset to defaults is a 40px box 14 + 18 (status line) + 6 under the last divider.
mod common;

use common::{at, open_sized, tmp};
use gpui_calendar::store::CALENDAR_NAMES;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};
use std::{cell::Cell, rc::Rc};

fn top(w: &gpui_kit::Window, id: &'static str) -> f32 {
    f32::from(w.find(id).bounds().top())
}

fn close(got: impl Into<f32>, want: f32, what: &str) {
    let got = got.into();
    assert!((got - want).abs() < 0.6, "{what}: {got} vs {want}");
}

#[gpui_kit::test]
fn appearance_rows_follow_the_mock_rhythm(cx: &mut TestAppContext) {
    let (h, _) = open_sized(cx, &tmp("mock-appearance"), (1440., 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click("account-menu", cx);
        w.render_frame(cx);
        // Mode (40 tall, 68 + 1px rule) to the Accent swatch: 40 + 14 + 1 + 14.
        close(top(w, "accent-Red") - top(w, "mode-light"), 69., "mode pill to accent swatch");
        // Accent swatch (36) to the Neutral button: 36 + 14 + 1 + 14.
        close(top(w, "neutral-Cool") - top(w, "accent-Red"), 65., "accent swatch to neutral button");
        let (a, b, c) = (top(w, "calendar-color-0-Red"), top(w, "calendar-color-1-Red"), top(w, "calendar-color-2-Red"));
        close(b - a, 36., "calendar sub-row pitch");
        close(c - b, 36., "calendar sub-row pitch");
        // The last swatch row's bottom + 14 is the divider; Reset's centre is 1 + 14 + 18 + 6 + 20 under it.
        let bottom = f32::from(w.find("calendar-color-2-Red").bounds().bottom());
        close(f32::from(w.find("reset-appearance").bounds().center().y) - bottom, 73., "reset centre under the last swatch row");
    })
    .unwrap();
}

/// Selected swatch (mock `box-shadow: 0 0 0 2px bg, 0 0 0 4px ink` on the 24px dot): a 32px outer circle, 2px ink border and 2px gap round the dot.
/// GPUI sizes are border-box, so the 32px ring box holds its border; the dot centred inside it leaves (32 - 24) / 2 = 4 = 2 border + 2 gap on each side.
#[gpui_kit::test]
fn selected_swatch_ring_is_a_32px_circle_with_a_2px_border_and_2px_gap(cx: &mut TestAppContext) {
    assert!(CALENDAR_NAMES.len() >= 3, "the rhythm test indexes calendars 0 to 2");
    let (h, _) = open_sized(cx, &tmp("mock-appearance-ring"), (1440., 900.), Rc::new(Cell::new(at(28, 17, 35))), |_| {});
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click("account-menu", cx);
        w.render_frame(cx);
        // The default Work calendar color is selected; so is the default accent.
        for id in ["calendar-color-0-Blue", "accent-Blue"] {
            let (swatch, ring, dot) = (w.find(id).bounds(), w.find(format!("{id}-ring")).bounds(), w.find(format!("{id}-dot")).bounds());
            close(f32::from(swatch.size.width), 36., "swatch target");
            close(f32::from(ring.size.width), 32., "ring outer width");
            close(f32::from(ring.size.height), 32., "ring outer height");
            close(f32::from(dot.size.width), 24., "dot");
            close(f32::from(dot.left() - ring.left()), 4., "border 2 + gap 2 on the left");
            close(f32::from(ring.right() - dot.right()), 4., "border 2 + gap 2 on the right");
            close(f32::from(dot.top() - ring.top()), 4., "border 2 + gap 2 on top");
            close(f32::from(ring.center().x - swatch.center().x), 0., "ring centred in its target");
        }
    })
    .unwrap();
}
