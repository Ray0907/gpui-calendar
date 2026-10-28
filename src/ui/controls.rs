//! Shared building blocks: colors, focus ring, pressable controls, calendar shapes, icons.
//! GPUI divs have no transform, so the 0.97 press scale of guide 8.3 is not available: every
//! control dims to 85% while pressed instead, which is the guide's rule for dense targets.

use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::{
    App, BoxShadow, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    Role, StatefulInteractiveElement, Styled, TestSupportExt, Window, canvas, div, point, px
};

use super::a11y::Disabled;
pub use crate::theme::gpui_adapter::{banner_scrim, banner_wash, elevation, hsla as c, on_color};

/// Guide 3 "Focus ring": 2px ring, 2px gap, drawn as two spread shadows so it follows the radius.
const RING_PX: f32 = 2.;
const RING_GAP_PX: f32 = 2.;

pub(super) fn ring(gap: Hsla, color: Hsla) -> Vec<BoxShadow> {
    let s = |color, spread| BoxShadow { color, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(spread), inset: false };
    vec![s(gap, RING_GAP_PX), s(color, RING_GAP_PX + RING_PX)]
}

/// A 1px outline drawn outside the box, so marking a cell never changes its size.
pub fn ring_mark(color: Hsla) -> BoxShadow {
    BoxShadow { color, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(1.), inset: false }
}

/// A focusable, nameable, pressable element. `gap` is the color behind it (for the ring's gap);
/// `ring_color` is the theme ring, or white on the banner.
pub fn control<I: Into<ElementId>, L: Into<gpui_kit::SharedString>>(
    id: I,
    role: Role,
    label: L,
    gap: Hsla,
    ring_color: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> impl StatefulInteractiveElement + Styled + ParentElement + IntoElement + use<I, L> {
    control_disabled(id, role, label, false, gap, ring_color, window, cx)
}

/// `control` whose accessibility node is aria-disabled when `disabled`; it stays focusable and clickable.
#[allow(clippy::too_many_arguments)]
pub fn control_disabled<I: Into<ElementId>, L: Into<gpui_kit::SharedString>>(
    id: I,
    role: Role,
    label: L,
    disabled: bool,
    gap: Hsla,
    ring_color: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> impl StatefulInteractiveElement + Styled + ParentElement + IntoElement + use<I, L> {
    let id = id.into();
    let handle = window.use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle()).read(cx).clone();
    Disabled::new(div().id(id), disabled)
        .test_support()
        .track_focus(&handle.tab_stop(true))
        .role(role)
        .aria_label(label)
        .cursor_pointer()
        .active(|s| s.opacity(0.85))
        .focus_visible(move |s| s.shadow(ring(gap, ring_color)))
}

pub fn icon(name: IconName, color: Hsla, size: f32) -> Icon {
    Icon::new(name).with_size(px(size)).text_color(color)
}

/// Eyebrow text: 11 / 600, `text-3`. (GPUI has no letter-spacing, so the +6% tracking is missing.)
pub fn eyebrow(t: &crate::theme::resolve::Theme, text: &'static str) -> impl IntoElement {
    div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(c(t.neutral.t3)).child(text.to_uppercase())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Circle,
    Square,
    Diamond,
}

pub const SHAPE_PX: f32 = 8.;
/// Half the box, so the diamond's corners touch the edges and stay inside.
pub const DIAMOND_RADIUS: f32 = SHAPE_PX / 2.;

/// Fixed shape per calendar id (guide 3): circle, rounded square, diamond. 8px.
pub fn shape_of(id: u8) -> Shape {
    match id {
        0 => Shape::Circle,
        1 => Shape::Square,
        _ => Shape::Diamond,
    }
}

pub fn shape(kind: Shape, color: Hsla) -> impl IntoElement {
    match kind {
        Shape::Circle => div().size(px(SHAPE_PX)).flex_none().rounded_full().bg(color).into_any_element(),
        Shape::Square => div().size(px(SHAPE_PX)).flex_none().rounded(px(2.)).bg(color).into_any_element(),
        Shape::Diamond => canvas(
            |_, _, _| (),
            move |b, _, window, _| {
                let (cx, cy, r) = (b.center().x, b.center().y, px(DIAMOND_RADIUS));
                let mut p = PathBuilder::fill();
                p.move_to(point(cx, cy - r));
                p.line_to(point(cx + r, cy));
                p.line_to(point(cx, cy + r));
                p.line_to(point(cx - r, cy));
                p.close();
                if let Ok(path) = p.build() {
                    window.paint_path(path, color);
                }
            },
        )
        .size(px(SHAPE_PX))
        .flex_none()
        .into_any_element(),
    }
}
