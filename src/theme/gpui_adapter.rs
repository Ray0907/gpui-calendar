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
use gpui_kit::{BoxShadow, Hsla, point, px, rgb};

use super::contrast::{Rgb, over};
use super::resolve::Theme;

/// `wash-hover` (design-tokens 2): `fg` (t1) at 6% over the control's backdrop.
pub fn wash_hover(fg: Rgb, backdrop: Rgb) -> Hsla {
    hsla(over(fg, 0.06, backdrop))
}

pub fn hsla(c: Rgb) -> Hsla {
    rgb(c.to_u32()).into()
}

/// Guide fixed marks that do not follow the theme: the check drawn on a colored fill.
pub fn on_color() -> Hsla {
    hsla(Rgb::WHITE)
}

/// A black scrim of the given opacity (banner button, shadows).
pub fn shade(alpha: f32) -> Hsla {
    hsla(Rgb(0, 0, 0)).opacity(alpha)
}

/// Banner button: the dark scrim behind the label and the light wash on its chevron chip.
pub fn banner_scrim() -> Hsla {
    shade(0.28)
}

pub fn banner_wash() -> Hsla {
    on_color().opacity(0.18)
}

/// Guide 5 elevation token: the soft shadow used on cards and the view switch.
pub fn elevation(t: &Theme) -> Vec<BoxShadow> {
    let (near, far) = if t.dark { (0.4, 0.5) } else { (0.04, 0.14) };
    let shadow = |alpha, y, blur, spread| BoxShadow { color: shade(alpha), offset: point(px(0.), px(y)), blur_radius: px(blur), spread_radius: px(spread), inset: false };
    vec![shadow(near, 1., 2., 0.), shadow(far, 10., 24., -10.)]
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
