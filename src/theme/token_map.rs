//! Every gpui-component `ThemeColor` field, assigned from a role (design-tokens.md section 4).
//!
//! `Theme::apply_semantic_tokens` writes only 17 of the library's fields; the rest would keep the library's
//! built-in theme. `theme_color_table` names every field once, so a test can compare the names with the library's
//! own (`ThemeColor` serializes, so its JSON keys are its fields). `theme_colors` lays the table over the library
//! default: a field the table forgets stays at the default and fails `token_coverage`, it does not crash the app.
//! Colors cross into the library as `#rrggbbaa`, so alpha is held to 8 bits.

use gpui_kit::Hsla;
use gpui_kit::component::ThemeColor;
use serde_json::Value;

use super::contrast::{Rgb, over, wash_hover_rgb, wash_press_rgb};
use super::gpui_adapter::{hsla, shade};
use super::palette::accent;
use super::resolve::Theme;
use crate::domain::event::CalendarId;
use crate::store::settings::Accent;

/// Scrim opacity of the `scrim` role: 18% light, 40% dark.
pub fn scrim_alpha(dark: bool) -> f32 {
    if dark { 0.40 } else { 0.18 }
}

/// The ink family: a fill, the text on it, hover at 90% and active at 80% over `page`. Never white, never `t1` raw.
struct Ink {
    fill: Rgb,
    on: Rgb,
    hover: Rgb,
    active: Rgb,
}

pub fn theme_color_table(t: &Theme) -> Vec<(&'static str, Hsla)> {
    let n = &t.neutral;
    let ink = Ink { fill: t.ink, on: t.on_ink, hover: over(t.ink, 0.9, n.page), active: over(t.ink, 0.8, n.page) };
    let mut v: Vec<(&'static str, Hsla)> = Vec::with_capacity(139);
    let mut put = |name: &'static str, c: Rgb| v.push((name, hsla(c)));
    // Base
    put("background", n.page);
    put("foreground", n.t1);
    put("popover", n.surface);
    put("popover_foreground", n.t1);
    put("primary", ink.fill);
    put("primary_foreground", ink.on);
    put("secondary", n.field);
    put("secondary_foreground", n.t1);
    put("muted", n.subtle);
    put("muted_foreground", n.t3);
    put("accent", n.subtle);
    put("accent_foreground", n.t1);
    put("border", n.line);
    put("input", n.border);
    put("ring", t.ring);
    // Date picker, input
    put("secondary_hover", wash_hover_rgb(n.t1, n.surface));
    put("secondary_active", wash_press_rgb(n.t1, n.surface));
    put("caret", ink.fill);
    put("primary_hover", ink.hover);
    put("primary_active", ink.active);
    // Status fills and danger: no status colors in the guide, so the ink family.
    for (fill, on, hover, active) in [
        ("danger", "danger_foreground", "danger_hover", "danger_active"),
        ("info", "info_foreground", "info_hover", "info_active"),
        ("success", "success_foreground", "success_hover", "success_active"),
        ("warning", "warning_foreground", "warning_hover", "warning_active"),
    ] {
        put(fill, ink.fill);
        put(on, ink.on);
        put(hover, ink.hover);
        put(active, ink.active);
    }
    // Buttons: ghost, then the ink family, then the secondary (field) button.
    put("button_foreground", n.t1);
    put("button_hover", wash_hover_rgb(n.t1, n.page));
    put("button_active", wash_press_rgb(n.t1, n.page));
    for (fill, on, hover, active) in [
        ("button_primary", "button_primary_foreground", "button_primary_hover", "button_primary_active"),
        ("button_danger", "button_danger_foreground", "button_danger_hover", "button_danger_active"),
        ("button_info", "button_info_foreground", "button_info_hover", "button_info_active"),
        ("button_success", "button_success_foreground", "button_success_hover", "button_success_active"),
        ("button_warning", "button_warning_foreground", "button_warning_hover", "button_warning_active"),
    ] {
        put(fill, ink.fill);
        put(on, ink.on);
        put(hover, ink.hover);
        put(active, ink.active);
    }
    put("button_secondary", n.field);
    put("button_secondary_foreground", n.t1);
    put("button_secondary_hover", wash_hover_rgb(n.t1, n.field));
    put("button_secondary_active", wash_press_rgb(n.t1, n.field));
    // Lists, tables, tabs, sidebar, window chrome: `surface`, washes over it, `line`.
    for name in ["list", "list_even", "list_head", "accordion", "group_box", "table", "table_even", "table_head", "table_foot", "tab", "tab_bar", "sidebar", "title_bar", "status_bar", "scrollbar", "switch_thumb"] {
        put(name, n.surface);
    }
    for name in ["list_hover", "table_hover", "sidebar_accent"] {
        put(name, wash_hover_rgb(n.t1, n.surface));
    }
    for name in ["list_active", "table_active", "description_list_label"] {
        put(name, n.subtle);
    }
    for name in ["list_active_border", "table_active_border", "sidebar_primary", "progress_bar", "slider_bar"] {
        put(name, ink.fill);
    }
    for name in ["table_head_foreground", "table_foot_foreground"] {
        put(name, n.t3);
    }
    for name in ["sidebar_foreground", "sidebar_accent_foreground", "tab_foreground", "tab_active_foreground"] {
        put(name, n.t1);
    }
    for name in ["table_row_border", "sidebar_border", "title_bar_border", "status_bar_border", "window_border", "chart_grid"] {
        put(name, n.line);
    }
    for name in ["group_box_foreground", "description_list_label_foreground", "link", "chart_bullish", "chart_bearish", "chart_5"] {
        put(name, n.t2);
    }
    for name in ["link_active", "link_hover"] {
        put(name, n.t1);
    }
    put("tab_active", n.page);
    put("tab_bar_segmented", n.field);
    put("switch", n.field);
    put("skeleton", n.field);
    put("sidebar_primary_foreground", ink.on);
    put("slider_thumb", ink.on);
    put("chart_4", n.t3);
    for (name, id) in [("chart_1", 0), ("chart_2", 1), ("chart_3", 2)] {
        put(name, cal_dot(t, id));
    }
    // Base hues: the app's accent presets by hue, so no foreign hue is reachable. The `_light` step is 20% over `surface`.
    for (name, light, preset) in [
        ("red", "red_light", Accent::Red),
        ("green", "green_light", Accent::Green),
        ("blue", "blue_light", Accent::Blue),
        ("yellow", "yellow_light", Accent::Orange),
        ("magenta", "magenta_light", Accent::Pink),
        ("cyan", "cyan_light", Accent::Teal),
    ] {
        let rgb = accent(preset).rgb;
        put(name, rgb);
        put(light, over(rgb, 0.2, n.surface));
    }
    // Alpha roles.
    let mut alpha = |name: &'static str, c: Hsla| v.push((name, c));
    alpha("selection", hsla(t.ink).opacity(0.25));
    alpha("drag_border", hsla(t.ink).opacity(0.65));
    alpha("drop_target", hsla(t.ink).opacity(0.20));
    alpha("scrollbar_thumb", hsla(n.t3).opacity(0.45));
    alpha("scrollbar_thumb_hover", hsla(n.t3).opacity(0.70));
    alpha("overlay", shade(scrim_alpha(t.dark)));
    alpha("button", hsla(n.page).opacity(0.0));
    v
}

/// The library default of this scheme with every table field written over it.
pub fn theme_colors(t: &Theme) -> ThemeColor {
    let base = if t.dark { *ThemeColor::dark() } else { *ThemeColor::light() };
    lay(base, theme_color_table(t))
}

/// `table` written over `base`, field by field. A name the library does not have, or a value it cannot read back, panics with the field name.
pub fn lay(base: ThemeColor, table: Vec<(&'static str, Hsla)>) -> ThemeColor {
    let Value::Object(mut fields) = serde_json::to_value(base).expect("ThemeColor serializes") else { panic!("ThemeColor is not an object") };
    for (name, c) in table {
        let slot = fields.get_mut(name).unwrap_or_else(|| panic!("token_map: `{name}` is not a ThemeColor field"));
        *slot = serde_json::to_value(c).unwrap_or_else(|e| panic!("token_map: `{name}` does not serialize: {e}"));
    }
    serde_json::from_value(Value::Object(fields)).expect("token_map: ThemeColor rejects the mapped fields")
}

/// The dot color of calendar `id`; a missing slot is a bug, not a gray (`chart_1..3`).
pub fn cal_dot(t: &Theme, id: CalendarId) -> Rgb {
    t.cal(id).unwrap_or_else(|| panic!("token_map: calendar {id} has no theme slot")).dot
}
