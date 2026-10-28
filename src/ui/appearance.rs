//! The Appearance settings screen (guide 3 "Appearance"): Mode, Accent, Neutral, per-calendar colors, Reset.

use gpui_kit::component::IconName;
use gpui_kit::{
    AnyElement, BoxShadow, ClickEvent, Context, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, TestSupportExt, Toggled, Window, div, point, px, prelude::*,
};

use super::controls::{c, control, control_disabled, focus_ring, icon, on_color, shape, shape_of};
use super::shell::Shell;
use crate::store::CALENDAR_NAMES;
use crate::store::settings::{Accent, CalendarColor, Mode, Neutral};
use crate::theme::palette::{ACCENTS, NEUTRALS, SLOTS, accent, neutral, slot};
use crate::theme::resolve::Theme;

/// The status line's name is its text, or "Status" while it is empty.
fn status_name(hint: &str) -> String {
    if hint.is_empty() { "Status".into() } else { hint.into() }
}

const MODES: [(Mode, &str); 3] = [(Mode::Light, "Light"), (Mode::Dark, "Dark"), (Mode::System, "System")];

/// One settings row: 160px label column (title + small help) and the controls.
fn row(t: &Theme, title: &'static str, help: &'static str, body: impl IntoElement) -> impl IntoElement {
    let n = &t.neutral;
    div()
        .flex()
        .items_center()
        .gap(px(20.))
        .py(px(10.))
        .border_t_1()
        .border_color(c(n.line))
        .child(
            div()
                .w(px(160.))
                .flex_none()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                .children((!help.is_empty()).then(|| div().text_size(px(12.)).text_color(c(n.t3)).child(help))),
        )
        .child(div().flex_1().child(body))
}

/// A swatch: 36px target, 24px dot; the selected one has a check and a 2px ink ring.
/// An unavailable swatch stays a focus target (guide: aria-disabled, not removed): its node is marked
/// disabled, its label says "unavailable: reason" once, and activating it only shows the reason.
struct Swatch {
    id: String,
    label: String,
    color: Hsla,
    selected: bool,
    unavailable: Option<String>,
}

impl Shell {
    fn swatch(&mut self, t: &Theme, s: Swatch, window: &mut Window, cx: &mut Context<Self>, on: impl Fn(&mut Self) + 'static) -> AnyElement {
        let n = &t.neutral;
        let spread = |color, px_| BoxShadow { color, offset: point(px(0.), px(0.)), blur_radius: px(0.), spread_radius: px(px_), inset: false };
        let dot = div()
            .size(px(24.))
            .rounded_full()
            .bg(s.color)
            .flex()
            .items_center()
            .justify_center()
            .when(s.selected, |d| d.shadow(vec![spread(c(n.surface), 2.), spread(c(t.ink), 4.)]))
            .children(s.selected.then(|| icon(IconName::Check, if t.dark { c(n.page) } else { on_color() }, 14.)));
        let unavailable = s.unavailable.is_some();
        control_disabled(s.id, Role::Button, s.label, unavailable, c(n.surface), c(t.ring), window, cx)
            .size(px(36.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .when(unavailable, |d| d.opacity(0.35))
            .aria_toggled(if s.selected { Toggled::True } else { Toggled::False })
            .child(dot)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                on(this);
                this.sync(cx);
            }))
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn choice_pill(&mut self, t: &Theme, id: String, label: &'static str, on: bool, window: &mut Window, cx: &mut Context<Self>, act: impl Fn(&mut Self) + 'static) -> AnyElement {
        let n = &t.neutral;
        control(id, Role::Button, label, c(n.surface), c(t.ring), window, cx)
            .h(px(40.))
            .px(px(16.))
            .flex()
            .items_center()
            .rounded_full()
            .border_1()
            .border_color(if on { c(t.ink) } else { c(n.border) })
            .text_size(px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .when(on, |d| d.bg(c(t.ink)).text_color(c(t.on_ink)).focus_visible(focus_ring(c(n.surface), c(t.ring), c(t.ink))))
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .child(label)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                act(this);
                this.sync(cx);
            }))
            .into_any_element()
    }

    fn neutral_button(&mut self, t: &Theme, nt: Neutral, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let n = &t.neutral;
        let on = self.model.store.settings.appearance.neutral == nt;
        let set = neutral(nt, t.dark);
        let chips = [set.page, set.t3, set.t1].map(|x| div().size(px(12.)).rounded_full().ml(px(-3.)).bg(c(x)).border_1().border_color(c(n.line)));
        control(format!("neutral-{nt:?}"), Role::Button, format!("{nt:?}"), c(n.surface), c(t.ring), window, cx)
            .h(px(40.))
            .px(px(14.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(10.))
            .border(px(if on { 2. } else { 1. }))
            .border_color(if on { c(t.ink) } else { c(n.border) })
            .when(on, |d| d.bg(c(n.subtle)))
            .font_weight(FontWeight::MEDIUM)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .child(div().id(format!("neutral-swatch-{nt:?}")).test_support().flex().pl(px(3.)).children(chips))
            .child(div().id(format!("neutral-label-{nt:?}")).test_support().child(format!("{nt:?}")))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.model.set_neutral(nt);
                this.sync(cx);
            }))
            .into_any_element()
    }

    pub(super) fn appearance_screen(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let n = &t.neutral;
        let a = self.model.store.settings.appearance;
        let modes: Vec<_> = MODES
            .iter()
            .map(|&(m, label)| self.choice_pill(t, format!("mode-{}", label.to_lowercase()), label, a.mode == m, window, cx, move |s| s.model.set_mode(m)))
            .collect();
        let accents: Vec<_> = ACCENTS
            .iter()
            .map(|&ac: &Accent| {
                let s = Swatch {
                    id: format!("accent-{ac:?}"),
                    label: self.model.accent_name(ac),
                    color: c(accent(ac).rgb),
                    selected: a.accent == ac,
                    unavailable: self.model.accent_reason(ac),
                };
                self.swatch(t, s, window, cx, move |this| {
                    let _ = this.model.set_accent(ac);
                })
            })
            .collect();
        let neutrals: Vec<_> = NEUTRALS.iter().map(|&nt| self.neutral_button(t, nt, window, cx)).collect();
        let cals: Vec<_> = (0..CALENDAR_NAMES.len() as u8).map(|id| self.calendar_color_row(t, id, window, cx)).collect();
        let hint = self.model.hint.clone();
        let reset = control("reset-appearance", Role::Button, "Reset to defaults", c(n.surface), c(t.ring), window, cx)
            .mt(px(6.))
            .py(px(4.))
            .w_auto()
            .text_color(c(n.t2))
            .underline()
            .child("Reset to defaults")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.model.reset_appearance();
                this.sync(cx);
            }));
        div()
            .id("appearance")
            .test_support()
            .w_full()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll() // a short window or a two-line notice scrolls instead of clipping Reset
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(760.))
                    .px(px(28.))
                    .pt(px(20.))
                    .pb(px(12.))
                    .child(div().text_size(px(20.)).font_weight(FontWeight::BOLD).child("Appearance"))
                    .child(
                        div()
                            .mt(px(4.))
                            .mb(px(14.))
                            .text_color(c(n.t2))
                            .child("Colors apply instantly and are saved on this computer. Every option is checked for contrast, so any combination stays readable."),
                    )
                    .child(row(t, "Mode", "", div().flex().gap(px(8.)).children(modes)))
                    .child(row(t, "Accent", "Marks today", div().flex().flex_wrap().gap(px(2.)).children(accents)))
                    .child(row(t, "Neutral", "Tint of the grays", div().flex().gap(px(8.)).children(neutrals)))
                    .child(row(t, "Calendars", "One color each. The shape stays fixed.", div().flex().flex_col().gap(px(8.)).children(cals)))
                    .child(row(
                        t,
                        "",
                        "",
                        div()
                            .child(div().id("status").test_support().role(Role::Status).aria_label(status_name(&hint)).min_h(px(18.)).text_size(px(12.)).text_color(c(n.t2)).child(hint.clone()))
                            .child(reset),
                    )),
            )
    }

    fn calendar_color_row(&mut self, t: &Theme, id: u8, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let name = CALENDAR_NAMES[usize::from(id)];
        let dot = t.cal(id).map(|k| c(k.dot)).unwrap_or(c(t.ink));
        let current = self.model.calendar_color(id);
        let swatches: Vec<_> = SLOTS
            .iter()
            .map(|&col: &CalendarColor| {
                let s = Swatch {
                    id: format!("calendar-color-{id}-{col:?}"),
                    label: format!("{name}: {}", self.model.calendar_color_name(id, col)),
                    color: c(slot(col).scheme(t.dark).dot),
                    selected: current == Some(col),
                    unavailable: self.model.calendar_color_reason(id, col),
                };
                self.swatch(t, s, window, cx, move |this| {
                    let _ = this.model.set_calendar_color(id, col);
                })
            })
            .collect();
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .w(px(140.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(shape(shape_of(id), dot))
                    .child(name),
            )
            .child(div().flex().flex_wrap().gap(px(2.)).children(swatches))
            .into_any_element()
    }
}
