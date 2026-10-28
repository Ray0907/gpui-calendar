pub mod domain;
pub mod store;
pub mod theme;
pub mod ui;

/// Icon source for the real app and the headless shots, so the two cannot drift.
pub fn assets() -> gpui_kit::assets::Assets {
    gpui_kit::assets::Assets
}

/// An on/off environment flag: unset, blank, "0" and "false" (any case, surrounding whitespace ignored) are off.
pub fn env_flag(v: Option<&std::ffi::OsStr>) -> bool {
    v.is_some_and(|v| !["", "0", "false"].iter().any(|off| v.to_string_lossy().trim().eq_ignore_ascii_case(off)))
}

use gpui_kit::{Context, IntoElement, Render, TestSupportExt, Window, div, prelude::*};

/// Spike view: a draggable-target "grid" cell used to prove headless drag and layout.
pub struct Spike {
    pub down: Option<gpui_kit::Point<gpui_kit::Pixels>>,
    pub last: Option<gpui_kit::Point<gpui_kit::Pixels>>,
    pub released: Option<gpui_kit::Point<gpui_kit::Pixels>>,
}

impl Spike {
    pub fn new() -> Self {
        Self { down: None, last: None, released: None }
    }
}

impl Default for Spike {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for Spike {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            div()
                .id("cell")
                .test_support()
                .w(gpui_kit::px(200.))
                .h(gpui_kit::px(120.))
                .on_mouse_down(
                    gpui_kit::MouseButton::Left,
                    cx.listener(|this, e: &gpui_kit::MouseDownEvent, _, cx| {
                        this.down = Some(e.position);
                        cx.notify();
                    }),
                )
                .on_mouse_move(cx.listener(|this, e: &gpui_kit::MouseMoveEvent, _, cx| {
                    if e.pressed_button.is_some() {
                        this.last = Some(e.position);
                        cx.notify();
                    }
                }))
                .on_mouse_up(
                    gpui_kit::MouseButton::Left,
                    cx.listener(|this, e: &gpui_kit::MouseUpEvent, _, cx| {
                        this.released = Some(e.position);
                        cx.notify();
                    }),
                )
                .child("calendar spike"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::env_flag;

    #[test]
    fn only_a_real_value_turns_a_flag_on() {
        for off in [None, Some(""), Some("0"), Some("false"), Some("FALSE"), Some(" 0"), Some("0\n"), Some("  ")] {
            assert!(!env_flag(off.map(std::ffi::OsStr::new)), "{off:?}");
        }
        for on in ["1", "true", "yes"] {
            assert!(env_flag(Some(std::ffi::OsStr::new(on))), "{on}");
        }
    }
}
