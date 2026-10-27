//! Contrast math must agree with the reference values in tests/fixtures/contrast_expected.json.
use crate::theme::contrast::*;

#[test]
fn appearance_hex_parses() {
    assert_eq!(Rgb::hex("#d92d20"), Rgb(0xd9, 0x2d, 0x20));
    assert_eq!(Rgb::hex("ffffff"), Rgb::WHITE);
}

#[test]
fn appearance_wcag_extremes() {
    assert!((contrast(Rgb::WHITE, Rgb(0, 0, 0)) - 21.0).abs() < 1e-9);
    assert!((contrast(Rgb::WHITE, Rgb::WHITE) - 1.0).abs() < 1e-9);
    // symmetric
    let (a, b) = (Rgb::hex("#3b6cf6"), Rgb::hex("#ffffff"));
    assert_eq!(contrast(a, b), contrast(b, a));
}

#[test]
fn appearance_matches_guide_measurements() {
    // design-guide: white on default Red accent 4.83; Red accent on dark surface 3.70.
    let red = Rgb::hex("#d92d20");
    assert!((contrast(Rgb::WHITE, red) - 4.83).abs() < 0.005);
    assert!((contrast(red, Rgb::hex("#131724")) - 3.70).abs() < 0.005);
    // text-1 on surface, light: guide minimum over surfaces is 15.73 (page is the tightest).
    assert!((contrast(Rgb::hex("#161b2b"), Rgb::hex("#f4f5f9")) - 15.73).abs() < 0.01);
}

#[test]
fn appearance_over_composites_and_rounds() {
    assert_eq!(
        over(Rgb(255, 255, 255), 0.0, Rgb(10, 20, 30)),
        Rgb(10, 20, 30)
    );
    assert_eq!(
        over(Rgb(255, 255, 255), 1.0, Rgb(10, 20, 30)),
        Rgb(255, 255, 255)
    );
    // 96*0.18 + 19*0.82 = 32.86 -> 33 (Blue dark tint over surface #131724 red channel)
    assert_eq!(over(Rgb(96, 140, 255), 0.18, Rgb(0x13, 0x17, 0x24)).0, 33);
}

#[test]
fn appearance_hue_distance_wraps() {
    assert_eq!(hue_distance(350, 10), 20);
    assert_eq!(hue_distance(29, 29), 0);
    assert_eq!(hue_distance(0, 180), 180);
    assert_eq!(hue_distance(265, 258), 7);
}
