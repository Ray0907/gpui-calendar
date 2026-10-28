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
    use crate::theme::gpui_adapter::shade;
    assert_eq!(shade(0.5), gpui_kit::black().opacity(0.5));
}

/// Design-tokens 2: `scrim` is black at 18% in light and 40% in dark; the banner button's scrim goes 28% to 40% on hover.
#[test]
fn scrim_and_banner_scrim_follow_the_table() {
    use crate::store::settings::Appearance;
    use crate::theme::gpui_adapter::{banner_scrim, banner_scrim_hover, scrim};
    use crate::theme::resolve::resolve;
    let a = Appearance::default();
    assert_eq!(scrim(&resolve(&a, false)), gpui_kit::black().opacity(0.18));
    assert_eq!(scrim(&resolve(&a, true)), gpui_kit::black().opacity(0.40));
    assert_eq!((banner_scrim(), banner_scrim_hover()), (gpui_kit::black().opacity(0.28), gpui_kit::black().opacity(0.40)));
}

/// The ink family (state table): hover is `ink` at 90% over `page`, pressed 80%, in both themes, and neither is the resting ink.
#[test]
fn ink_hover_and_press_are_ninety_and_eighty_percent_over_page() {
    use crate::store::settings::Appearance;
    use crate::theme::contrast::over;
    use crate::theme::gpui_adapter::hsla;
    use crate::theme::resolve::resolve;
    use crate::ui::controls::{ink_hover_color, ink_press_color};
    for dark in [false, true] {
        let t = resolve(&Appearance::default(), dark);
        let (h, p) = (ink_hover_color(&t), ink_press_color(&t));
        assert_eq!((h, p), (hsla(over(t.ink, 0.9, t.neutral.page)), hsla(over(t.ink, 0.8, t.neutral.page))));
        assert!(h != hsla(t.ink) && p != h, "dark={dark}: rest, hover and pressed must differ");
    }
}
