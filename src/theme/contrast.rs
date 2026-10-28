//! WCAG 2.x contrast math, checked against the reference values in `tests/fixtures/contrast_expected.json`. No GPUI import.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const WHITE: Rgb = Rgb(255, 255, 255);

    /// `#rrggbb` or `rrggbb`. A const fn so palette tables can be written as hex strings;
    /// a bad literal fails the build.
    pub const fn hex(s: &str) -> Rgb {
        let b = s.as_bytes();
        let o = if !b.is_empty() && b[0] == b'#' { 1 } else { 0 };
        assert!(b.len() == o + 6, "hex colour must be 6 digits");
        Rgb(
            byte(b[o], b[o + 1]),
            byte(b[o + 2], b[o + 3]),
            byte(b[o + 4], b[o + 5]),
        )
    }

    pub fn luminance(self) -> f64 {
        let lin = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
    }

    /// `0xRRGGBB`, for handing to a UI toolkit.
    pub fn to_u32(self) -> u32 {
        u32::from(self.0) << 16 | u32::from(self.1) << 8 | u32::from(self.2)
    }
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("bad hex digit"),
    }
}

const fn byte(hi: u8, lo: u8) -> u8 {
    nibble(hi) << 4 | nibble(lo)
}

/// WCAG contrast ratio, 1.0 to 21.0. Symmetric.
pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la < lb { (lb, la) } else { (la, lb) };
    (hi + 0.05) / (lo + 0.05)
}

/// `fg` at `alpha` composited over opaque `bg`, rounded per channel (ties to even, like Python's `round`).
pub fn over(fg: Rgb, alpha: f64, bg: Rgb) -> Rgb {
    let mix = |f: u8, b: u8| {
        (f64::from(f) * alpha + f64::from(b) * (1.0 - alpha)).round_ties_even() as u8
    };
    Rgb(mix(fg.0, bg.0), mix(fg.1, bg.1), mix(fg.2, bg.2))
}

/// `wash-hover` and `wash-press` (design-tokens 2): the text color at 6% and 10% over the control's backdrop.
pub fn wash_hover_rgb(fg: Rgb, backdrop: Rgb) -> Rgb {
    over(fg, 0.06, backdrop)
}

pub fn wash_press_rgb(fg: Rgb, backdrop: Rgb) -> Rgb {
    over(fg, 0.10, backdrop)
}

/// Shortest distance between two hues on the 360 degree circle.
pub fn hue_distance(a: u16, b: u16) -> u16 {
    let d = a.abs_diff(b) % 360;
    d.min(360 - d)
}
