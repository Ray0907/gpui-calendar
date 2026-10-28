use crate::store::settings::{Appearance, Mode};
use crate::theme::contrast::Rgb;
use crate::theme::gpui_adapter::{color_tokens, hsla};
use crate::theme::palette::NEUTRALS;
use crate::theme::resolve::resolve;

#[test]
fn appearance_adapter_maps_every_token() {
    for mode in [Mode::Light, Mode::Dark] {
        for neutral in NEUTRALS {
            let t = resolve(
                &Appearance {
                    mode,
                    neutral,
                    ..Appearance::default()
                },
                false,
            );
            let (k, n) = (color_tokens(&t), &t.neutral);
            let ctx = format!("{mode:?} {neutral:?}");
            assert_eq!(k.background, hsla(n.page), "background {ctx}");
            assert_eq!(k.foreground, hsla(n.t1), "foreground {ctx}");
            assert_eq!(k.surface, hsla(n.surface), "surface {ctx}");
            assert_eq!(k.surface_foreground, hsla(n.t1), "surface_foreground {ctx}");
            assert_eq!(k.primary, hsla(t.ink), "primary {ctx}");
            assert_eq!(
                k.primary_foreground,
                hsla(t.on_ink),
                "primary_foreground {ctx}"
            );
            assert_eq!(k.secondary, hsla(n.field), "secondary {ctx}");
            assert_eq!(
                k.secondary_foreground,
                hsla(n.t1),
                "secondary_foreground {ctx}"
            );
            assert_eq!(k.muted, hsla(n.subtle), "muted {ctx}");
            assert_eq!(k.muted_foreground, hsla(n.t3), "muted_foreground {ctx}");
            assert_eq!(k.accent, hsla(n.subtle), "accent {ctx}");
            assert_eq!(k.accent_foreground, hsla(n.t1), "accent_foreground {ctx}");
            assert_eq!(k.border, hsla(n.line), "border {ctx}");
            assert_eq!(k.input, hsla(n.border), "input {ctx}");
            assert_eq!(k.ring, hsla(t.ring), "ring {ctx}");
            assert_eq!(k.destructive, hsla(t.ink), "destructive {ctx}");
            assert_eq!(k.destructive_foreground, hsla(t.on_ink), "destructive_foreground {ctx}");
            assert_eq!(k.selection, hsla(t.ink).opacity(0.25), "selection {ctx}");
        }
    }
}

#[test]
fn appearance_adapter_source_colors_differ_so_a_swapped_mapping_fails() {
    for neutral in NEUTRALS {
        for mode in [Mode::Light, Mode::Dark] {
            let n = resolve(
                &Appearance {
                    mode,
                    neutral,
                    ..Appearance::default()
                },
                false,
            )
            .neutral;
            // Light `field` is the same white as `surface` by design, so only dark can tell those two apart.
            let src = [n.page, n.surface, n.subtle, n.line, n.border, n.t1, n.t3];
            let src = if mode == Mode::Dark {
                [&src[..], &[n.field]].concat()
            } else {
                src.to_vec()
            };
            for (i, a) in src.iter().enumerate() {
                for b in &src[i + 1..] {
                    assert_ne!(a, b, "{mode:?} {neutral:?}");
                }
            }
        }
    }
}

#[test]
fn appearance_adapter_hsla_converts_rgb() {
    let red = hsla(Rgb(255, 0, 0));
    assert_eq!((red.h, red.s, red.l, red.a), (0.0, 1.0, 0.5, 1.0));
    let grey = hsla(Rgb(0x80, 0x80, 0x80));
    assert_eq!((grey.s, grey.a), (0.0, 1.0));
    assert!((grey.l - 128.0 / 255.0).abs() < 1e-6);
    assert_eq!(hsla(Rgb::WHITE).l, 1.0);
    assert_eq!(hsla(Rgb(0, 0, 0)).l, 0.0);
}
