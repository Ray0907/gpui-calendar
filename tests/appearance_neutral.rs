//! Headless test: each tint option draws its two-dot swatch left of its label (mock v7).
mod common;

use common::{open, tmp};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};

#[gpui_kit::test]
fn the_tint_swatch_sits_left_of_the_label(cx: &mut TestAppContext) {
    let (h, _) = open(cx, &tmp("neutral-swatch"), 1280.);
    cx.update_window(h, |_, w, cx| {
        w.render_frame(cx);
        w.click("account-menu", cx);
        w.render_frame(cx);
        for n in ["Cool", "Neutral", "Warm"] {
            let swatch = w.find(format!("neutral-swatch-{n}")).bounds();
            let label = w.find(format!("neutral-label-{n}")).bounds();
            assert!(swatch.left() < label.left(), "{n}: swatch x {:?} must be left of label x {:?}", swatch.left(), label.left());
        }
    })
    .unwrap();
}
