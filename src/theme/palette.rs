//! Preset tables for the approved palette; `tests/fixtures/palette_out.json` is generated from it and compared in `tests/appearance_fixture.rs`.
//! No GPUI import, no runtime file reads, and these tables are the only place colour literals live.

use super::contrast::{Rgb, over};
use crate::store::settings::{Accent, CalendarColor, Neutral};

pub const ACCENTS: [Accent; 8] = [
    Accent::Red,
    Accent::Orange,
    Accent::Green,
    Accent::Teal,
    Accent::Blue,
    Accent::Indigo,
    Accent::Violet,
    Accent::Pink,
];
pub const SLOTS: [CalendarColor; 8] = [
    CalendarColor::Red,
    CalendarColor::Orange,
    CalendarColor::Amber,
    CalendarColor::Green,
    CalendarColor::Teal,
    CalendarColor::Blue,
    CalendarColor::Violet,
    CalendarColor::Pink,
];
pub const NEUTRALS: [Neutral; 3] = [Neutral::Cool, Neutral::Neutral, Neutral::Warm];

/// Gray roles of one tint in one theme. `ink` is `t1`; `on_ink` is the surface (light) or page (dark).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeutralSet {
    pub page: Rgb,
    pub surface: Rgb,
    pub subtle: Rgb,
    pub field: Rgb,
    pub line: Rgb,
    pub border: Rgb,
    pub t1: Rgb,
    pub t2: Rgb,
    pub t3: Rgb,
}

/// "Next up" banner: gradient `from` to `to`, same in both themes. The value text is white.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Banner {
    pub from: Rgb,
    pub to: Rgb,
    pub label: Rgb,
    pub value: Rgb,
    pub secondary: Rgb,
}

/// The scrim and shadow color: black, alpha set by the role that uses it.
pub const SCRIM: Rgb = Rgb::hex("#000000");

/// The sidebar avatar's gradient stops (135 degrees), the same in both themes and every tint.
pub const AVATAR: [Rgb; 2] = [Rgb::hex("#a78bfa"), Rgb::hex("#60a5fa")];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    pub rgb: Rgb,
    pub alpha: f64,
}

impl Tint {
    pub fn over(self, surface: Rgb) -> Rgb {
        over(self.rgb, self.alpha, surface)
    }
}

/// One calendar colour in one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotScheme {
    pub dot: Rgb,
    pub tint: Tint,
    pub text: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub light: SlotScheme,
    pub dark: SlotScheme,
    /// OKLCH hue in degrees, used by the collision rules.
    pub hue: u16,
}

impl Slot {
    pub fn scheme(&self, dark: bool) -> &SlotScheme {
        if dark { &self.dark } else { &self.light }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPreset {
    pub rgb: Rgb,
    pub hue: u16,
}

pub fn accent(a: Accent) -> AccentPreset {
    let (hex, hue) = match a {
        Accent::Red => ("#d92d20", 29),
        Accent::Orange => ("#bf5400", 48),
        Accent::Green => ("#03853c", 150),
        Accent::Teal => ("#01817c", 190),
        Accent::Blue => ("#116fe8", 258),
        Accent::Indigo => ("#6262ea", 278),
        Accent::Violet => ("#9252da", 302),
        Accent::Pink => ("#cc378c", 350),
    };
    AccentPreset {
        rgb: Rgb::hex(hex),
        hue,
    }
}

const fn light(dot: &str, tint: &str, text: &str) -> SlotScheme {
    SlotScheme {
        dot: Rgb::hex(dot),
        tint: Tint {
            rgb: Rgb::hex(tint),
            alpha: 1.0,
        },
        text: Rgb::hex(text),
    }
}

const fn dark(dot: &str, tint: (u8, u8, u8, f64), text: &str) -> SlotScheme {
    SlotScheme {
        dot: Rgb::hex(dot),
        tint: Tint {
            rgb: Rgb(tint.0, tint.1, tint.2),
            alpha: tint.3,
        },
        text: Rgb::hex(text),
    }
}

pub fn slot(c: CalendarColor) -> &'static Slot {
    match c {
        CalendarColor::Red => {
            const {
                &Slot {
                    light: light("#e15b4d", "#ffebe8", "#6a2d25"),
                    dark: dark("#e25d4d", (248, 120, 104, 0.16), "#fec4bb"),
                    hue: 29,
                }
            }
        }
        CalendarColor::Orange => {
            const {
                &Slot {
                    light: light("#d16c00", "#ffece0", "#663305"),
                    dark: dark("#d16c00", (239, 133, 46, 0.16), "#fec7a2"),
                    hue: 55,
                }
            }
        }
        CalendarColor::Amber => {
            const {
                &Slot {
                    light: light("#b07c00", "#fdf2d0", "#713f12"),
                    dark: dark("#e0a800", (250, 204, 21, 0.16), "#f5dc86"),
                    hue: 80,
                }
            }
        }
        CalendarColor::Green => {
            const {
                &Slot {
                    light: light("#1f9d55", "#e4f5ea", "#14532d"),
                    dark: dark("#22a55b", (52, 211, 120, 0.16), "#9be8b9"),
                    hue: 152,
                }
            }
        }
        CalendarColor::Teal => {
            const {
                &Slot {
                    light: light("#029697", "#daf7f6", "#024d4d"),
                    dark: dark("#039797", (8, 188, 188, 0.16), "#93e5e4"),
                    hue: 195,
                }
            }
        }
        CalendarColor::Blue => {
            const {
                &Slot {
                    light: light("#3b6cf6", "#e8efff", "#1e3a8a"),
                    dark: dark("#4d7cff", (96, 140, 255, 0.18), "#b4ccff"),
                    hue: 265,
                }
            }
        }
        CalendarColor::Violet => {
            const {
                &Slot {
                    light: light("#9d6fe3", "#f2edfe", "#4a366b"),
                    dark: dark("#9e70e3", (181, 139, 249, 0.16), "#dbcafe"),
                    hue: 300,
                }
            }
        }
        CalendarColor::Pink => {
            const {
                &Slot {
                    light: light("#d65b9c", "#ffe9f3", "#642c49"),
                    dark: dark("#d75c9d", (237, 118, 179, 0.16), "#febfdc"),
                    hue: 350,
                }
            }
        }
    }
}

const fn set(v: [&str; 9]) -> NeutralSet {
    NeutralSet {
        page: Rgb::hex(v[0]),
        surface: Rgb::hex(v[1]),
        subtle: Rgb::hex(v[2]),
        field: Rgb::hex(v[3]),
        line: Rgb::hex(v[4]),
        border: Rgb::hex(v[5]),
        t1: Rgb::hex(v[6]),
        t2: Rgb::hex(v[7]),
        t3: Rgb::hex(v[8]),
    }
}

pub fn neutral(n: Neutral, dark: bool) -> &'static NeutralSet {
    match (n, dark) {
        (Neutral::Cool, false) => {
            const {
                &set([
                    "#f4f5f9", "#ffffff", "#f8f9fc", "#ffffff", "#eceef4", "#8891a5", "#161b2b",
                    "#3f485c", "#636d83",
                ])
            }
        }
        (Neutral::Cool, true) => {
            const {
                &set([
                    "#0b0d14", "#131724", "#161a28", "#1c2132", "#212637", "#6f7893", "#eef0f8",
                    "#bcc3d6", "#8a93ad",
                ])
            }
        }
        (Neutral::Neutral, false) => {
            const {
                &set([
                    "#f5f5f5", "#ffffff", "#f9f9f9", "#ffffff", "#eeeeee", "#919191", "#1c1c1c",
                    "#484848", "#6d6d6d",
                ])
            }
        }
        (Neutral::Neutral, true) => {
            const {
                &set([
                    "#0d0d0d", "#181818", "#1b1b1b", "#222222", "#272727", "#797979", "#f0f0f0",
                    "#c3c3c3", "#949494",
                ])
            }
        }
        (Neutral::Warm, false) => {
            const {
                &set([
                    "#f6f5f3", "#ffffff", "#faf9f8", "#ffffff", "#f0eeeb", "#979087", "#211b14",
                    "#4f473e", "#746b62",
                ])
            }
        }
        (Neutral::Warm, true) => {
            const {
                &set([
                    "#100d0a", "#1c1711", "#1f1a14", "#272119", "#2c261e", "#81776b", "#f3f0ed",
                    "#c9c2ba", "#9c9287",
                ])
            }
        }
    }
}

const fn banner_of(from: &str, to: &str, label: &str, secondary: &str) -> Banner {
    Banner {
        from: Rgb::hex(from),
        to: Rgb::hex(to),
        label: Rgb::hex(label),
        value: Rgb::WHITE,
        secondary: Rgb::hex(secondary),
    }
}

pub fn banner(n: Neutral) -> &'static Banner {
    match n {
        Neutral::Cool => const { &banner_of("#161b2b", "#2a3149", "#a9b8e8", "#c4cdea") },
        Neutral::Neutral => const { &banner_of("#1c1c1c", "#323232", "#bebebe", "#d1d1d1") },
        Neutral::Warm => const { &banner_of("#211b14", "#3a3126", "#cdbaa6", "#dacfc3") },
    }
}
