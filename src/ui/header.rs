//! Main card header: title, Day/Week/Month/Year switch, previous / Today / next.

use gpui_kit::component::IconName;
use gpui_kit::{
    ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role, StatefulInteractiveElement, Styled, TestSupportExt,
    Toggled, Window, div, px, prelude::*,
};

use super::controls::{c, control, elevation, focus_ring, icon};
use super::model::{Command, View};
use super::shell::Shell;
use crate::theme::resolve::Theme;

/// Height of the header row: the year view sizes its days from what the window has left.
pub const HEADER_H: f32 = 64.;

impl Shell {
    pub(super) fn header(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let n = &t.neutral;
        let (month, year) = self.model.title();
        let current = self.model.view;
        let seg: Vec<_> = View::ALL
            .iter()
            .map(|&v| {
                let on = v == current;
                control(format!("view-{}", v.name().to_lowercase()), Role::Button, v.name(), c(n.field), c(t.ring), window, cx)
                    .h(px(34.))
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .text_size(px(12.))
                    .text_color(if on { c(n.t1) } else { c(n.t2) })
                    .font_weight(if on { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                    .when(on, |s| s.bg(c(n.page)).focus_visible(focus_ring(c(n.field), c(t.ring), c(n.page))))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .child(v.name())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.run(Command::Show(v), cx);
                    }))
            })
            .collect();
        let prev_name = self.model.step_name(false);
        let next_name = self.model.step_name(true);
        let step = |id: &'static str, name: String, ic: IconName, dir: i32, window: &mut Window, cx: &mut Context<Self>| {
            control(id, Role::Button, name, c(n.surface), c(t.ring), window, cx)
                .w(px(44.))
                .h(px(40.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(c(n.page))
                .focus_visible(focus_ring(c(n.surface), c(t.ring), c(n.page)))
                .child(icon(ic, c(n.t1), 16.))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.run(Command::Step(dir), cx);
                }))
        };
        let prev = step("nav-prev", prev_name, IconName::ChevronLeft, -1, window, cx);
        let next = step("nav-next", next_name, IconName::ChevronRight, 1, window, cx);
        let today = control("nav-today", Role::Button, "Today", c(n.surface), c(t.ring), window, cx)
            .h(px(40.))
            .px(px(16.))
            .flex()
            .items_center()
            .rounded_full()
            .bg(c(n.page))
            .focus_visible(focus_ring(c(n.surface), c(t.ring), c(n.page)))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .child("Today")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.run(Command::Today, cx);
            }));
        div()
            .id("header")
            .test_support()
            .flex()
            .items_center()
            .px(px(16.))
            .h(px(HEADER_H))
            .flex_none()
            .child(
                div()
                    .id("header-title")
                    .test_support()
                    .aria_label(month.as_ref().map_or(year.clone(), |m| format!("{m} {year}")))
                    .flex_1()
                    // The title gives way first: without a zero minimum width the controls are pushed off a narrow window.
                    .min_w(px(0.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .text_size(px(24.))
                    .font_weight(FontWeight::BOLD)
                    .when(month.is_some(), |s| s.children(month.clone()))
                    // Alone (Year view) the year is the title: text-1.
                    .child(if month.is_some() { div().text_color(c(n.t3)).font_weight(FontWeight::MEDIUM).child(year) } else { div().child(year) }),
            )
            .child(
                div()
                    .id("view-switch")
                    // A segmented control owns Left and Right while it has focus: they never step the period.
                    .on_key_down(|e: &KeyDownEvent, _, cx| {
                        if !e.keystroke.modifiers.modified() && matches!(e.keystroke.key.as_str(), "left" | "right") {
                            cx.stop_propagation();
                        }
                    })
                    .flex()
                    .gap(px(2.))
                    .p(px(3.))
                    .rounded_full()
                    .bg(c(n.field))
                    .shadow(elevation(t))
                    .children(seg),
            )
            .child(div().flex_1().flex().justify_end().gap(px(8.)).child(prev).child(today).child(next))
            .into_any_element()
    }
}
