//! Shared building blocks: colors, focus ring, pressable controls, calendar shapes, icons.
//! GPUI divs have no transform, so the 0.97 press scale of guide 8.3 is not available: every
//! control dims to 85% while pressed instead, which is the guide's rule for dense targets.

use gpui_kit::component::{Icon, IconName, Sizable};
use gpui_kit::{
    App, BoxShadow, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    Pixels, Role, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, TestSupportExt, Window, canvas, div, point, prelude::*, px
};

use super::a11y::Disabled;
use crate::theme::contrast::{Rgb, over, wash_press_rgb};
use crate::theme::resolve::Theme;
pub use crate::theme::gpui_adapter::{banner_scrim, banner_scrim_hover, banner_wash, elevation, hsla as c, scrim, wash_hover};

/// Guide 3 "Focus ring": 2px ring, 2px gap, drawn as two spread shadows so it follows the radius.
const RING_PX: f32 = 2.;
const RING_GAP_PX: f32 = 2.;

pub(super) fn ring(gap: Hsla, color: Hsla) -> Vec<BoxShadow> {
    let s = |color, spread| BoxShadow { color, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(spread), inset: false };
    vec![s(gap, RING_GAP_PX), s(color, RING_GAP_PX + RING_PX)]
}

/// The focus-visible style: the ring plus a fill of `fill`, because a shadow shows through a box with no fill of its own.
/// A control with its own fill passes it again via `.focus_visible(focus_ring(gap, ring_color, fill))`.
pub(super) fn focus_ring(gap: Hsla, ring_color: Hsla, fill: Hsla) -> impl FnOnce(StyleRefinement) -> StyleRefinement {
    move |s| s.bg(fill).shadow(ring(gap, ring_color))
}

/// A 1px outline drawn outside the box, so marking a cell never changes its size.
pub fn ring_mark(color: Hsla) -> BoxShadow {
    BoxShadow { color, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(1.), inset: false }
}

/// Field hover (state table, Field): the outline redraws in `t2`. The field's own border is the library's, so this is an overlay
/// shown only while the box `id` is hovered. `live` is false while the field is focused (the ring wins), disabled or in error.
pub(super) fn field_hover(t: &crate::theme::resolve::Theme, id: impl Into<SharedString>, radius: Pixels, live: bool, field: impl IntoElement) -> impl IntoElement {
    let id = id.into();
    let outline = div().absolute().inset_0().rounded(radius).border_1().border_color(c(t.neutral.t2)).invisible().group_hover(id.clone(), |s| s.visible());
    div().id(id.clone()).test_support().group(id).relative().child(field).when(live, |d| d.child(outline))
}

/// Hover of a neutral control (state table, ghost and chip): `wash-hover`, `fg` at 6% over the fill `backdrop` it sits on. Pressed keeps
/// `control`'s 85% dim. Never `ink`, `t1` or white as a fill.
pub(super) fn hover_wash(fg: Rgb, backdrop: Rgb) -> impl FnOnce(StyleRefinement) -> StyleRefinement + use<> {
    let wash = wash_hover(fg, backdrop);
    move |s| s.bg(wash)
}

/// `hover_wash` in the text color: the ghost button, day cell, segment and chip hover.
pub(super) fn ghost_hover(t: &Theme, backdrop: Rgb) -> impl FnOnce(StyleRefinement) -> StyleRefinement + use<> {
    hover_wash(t.neutral.t1, backdrop)
}

/// Pressed fill of a list row (`wash-press`, `t1` at 10%): a fill, not a dim, so it replaces `control`'s 85%.
pub(super) fn row_press(t: &Theme, backdrop: Rgb) -> impl FnOnce(StyleRefinement) -> StyleRefinement + use<> {
    let wash = c(wash_press_rgb(t.neutral.t1, backdrop));
    move |s| s.bg(wash)
}

/// Filled ink controls (add, Save, selected pill): hover is `ink` at 90% over `page`, pressed 80% (design-states 3). In dark that dims the near-white fill.
pub(super) fn ink_hover_color(t: &Theme) -> Hsla {
    c(over(t.ink, 0.9, t.neutral.page))
}

pub(super) fn ink_press_color(t: &Theme) -> Hsla {
    c(over(t.ink, 0.8, t.neutral.page))
}

pub(super) fn ink_hover(t: &Theme) -> impl FnOnce(StyleRefinement) -> StyleRefinement + use<> {
    let fill = ink_hover_color(t);
    move |s| s.bg(fill)
}

/// Replaces `control`'s 85% dim with the 80% fill, which is the table's pressed value for a filled control.
pub(super) fn ink_press(t: &Theme) -> impl FnOnce(StyleRefinement) -> StyleRefinement + use<> {
    let fill = ink_press_color(t);
    move |s| s.bg(fill)
}

/// A radio card (calendar card, neutral tint): the outline goes from `border` to `t2` on hover.
pub(super) fn card_hover(t: &Theme) -> impl FnOnce(StyleRefinement) -> StyleRefinement + use<> {
    let outline = c(t.neutral.t2);
    move |s| s.border_color(outline)
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
        // Disabled takes no press: `.active` would also replace the disabled 50% the caller sets.
        .when(!disabled, |d| d.active(|s| s.opacity(0.85)))
        .focus_visible(focus_ring(gap, ring_color, gap))
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
    shape_sized(kind, color, SHAPE_PX)
}

/// `shape` in a box of `size` px (the Year view's dots are smaller).
pub fn shape_sized(kind: Shape, color: Hsla, size: f32) -> impl IntoElement {
    match kind {
        Shape::Circle => div().size(px(size)).flex_none().rounded_full().bg(color).into_any_element(),
        Shape::Square => div().size(px(size)).flex_none().rounded(px((size / 4.).min(2.))).bg(color).into_any_element(),
        Shape::Diamond => canvas(
            |_, _, _| (),
            move |b, _, window, _| {
                let (cx, cy, r) = (b.center().x, b.center().y, px(DIAMOND_RADIUS * size / SHAPE_PX));
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
        .size(px(size))
        .flex_none()
        .into_any_element(),
    }
}
