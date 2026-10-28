//! Theme and Appearance model. Everything except `gpui_adapter` is pure: no GPUI import.
//!
//! `palette` holds the measured presets, `contrast` the WCAG math, `rules` the collision and
//! repair rules, `resolve` turns an `Appearance` into concrete colors, and `gpui_adapter` maps
//! that onto gpui-kit's theme tokens.

pub mod contrast;
pub mod gpui_adapter;
pub mod palette;
pub mod resolve;
pub mod rules;
pub mod token_map;

#[cfg(test)]
mod tests;
