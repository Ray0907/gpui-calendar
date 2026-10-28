//! Pure geometry of the shared controls: focus ring and calendar shapes.
use gpui_kit::px;

use crate::ui::controls::{DIAMOND_RADIUS, SHAPE_PX, ring};

#[test]
fn focus_ring_is_two_px_thick_with_a_two_px_gap() {
    let s = ring(gpui_kit::white(), gpui_kit::black());
    assert_eq!(s.len(), 2);
    assert_eq!(s[0].spread_radius, px(2.), "gap: 2px offset from the edge");
    assert_eq!(s[1].spread_radius, px(4.), "ring: ends 2px after the gap");
    assert_eq!(s[1].color, gpui_kit::black());
    assert_eq!(s[0].color, gpui_kit::white());
    assert!(s.iter().all(|b| b.blur_radius == px(0.) && !b.inset));
}

#[test]
fn the_diamond_fits_its_box() {
    const { assert!(SHAPE_PX == 8. && DIAMOND_RADIUS * 2. <= SHAPE_PX, "the diamond must fit the 8px box") };
}

#[test]
fn the_system_dark_flag_has_one_definition() {
    use gpui_kit::WindowAppearance::*;
    let dark = |a| crate::ui::shell::is_dark(a);
    assert!(!dark(Light) && !dark(VibrantLight));
    assert!(dark(Dark) && dark(VibrantDark));
}

#[test]
fn fixed_marks_come_from_the_theme_module() {
    use crate::theme::gpui_adapter::{on_color, shade};
    assert_eq!(on_color(), gpui_kit::white());
    assert_eq!(shade(0.5), gpui_kit::black().opacity(0.5));
}
