//! Maps the resolved theme onto gpui-kit's `ColorTokens`. The only theme file that imports gpui-kit.
//!
//! Mapped: background, foreground, surface(+foreground), primary(+foreground) = ink, secondary
//! (+foreground) = field, muted(+foreground), accent(+foreground) = subtle hover wash, border = line,
//! input = field outline, ring.
//! Not mapped, left at gpui-kit's light/dark default: `destructive`, `destructive_foreground`,
//! `selection` (the guide has no such roles). The calendar accent (today marker), calendar colors and
//! the banner have no gpui-kit slot; views read them from `Theme`. Applying the tokens to the global
//! `Theme` needs a running `App` (`Theme::apply_semantic_tokens`) and is not done here.

use gpui_kit::base::ColorTokens;
use gpui_kit::{Hsla, rgb};

use super::contrast::Rgb;
use super::resolve::Theme;

pub fn hsla(c: Rgb) -> Hsla {
    rgb(c.to_u32()).into()
}

pub fn color_tokens(t: &Theme) -> ColorTokens {
    let n = &t.neutral;
    let base = if t.dark {
        ColorTokens::dark()
    } else {
        ColorTokens::light()
    };
    ColorTokens {
        background: hsla(n.page),
        foreground: hsla(n.t1),
        surface: hsla(n.surface),
        surface_foreground: hsla(n.t1),
        primary: hsla(t.ink),
        primary_foreground: hsla(t.on_ink),
        secondary: hsla(n.field),
        secondary_foreground: hsla(n.t1),
        muted: hsla(n.subtle),
        muted_foreground: hsla(n.t3),
        accent: hsla(n.subtle),
        accent_foreground: hsla(n.t1),
        border: hsla(n.line),
        input: hsla(n.border),
        ring: hsla(t.ring),
        ..base
    }
}
