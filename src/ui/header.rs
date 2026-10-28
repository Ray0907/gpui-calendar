//! Main card header: title, Day/Week/Month/Year switch, previous / Today / next.

use gpui_kit::component::IconName;
use gpui_kit::{
    ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Role, StatefulInteractiveElement, Styled, TestSupportExt,
    Toggled, Window, div, px, prelude::*,
};

use super::controls::{c, control, elevation, icon};
use super::model::View;
use super::shell::Shell;
use crate::theme::resolve::Theme;

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
                    .when(on, |s| s.bg(c(n.page)))
                    .aria_toggled(if on { Toggled::True } else { Toggled::False })
                    .child(v.name())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.model.set_view(v);
                        this.sync(cx);
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
                .child(icon(ic, c(n.t1), 16.))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.model.step(dir);
                    this.sync(cx);
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
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .child("Today")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.model.go_today();
                this.sync(cx);
            }));
        div()
            .id("header")
            .test_support()
            .flex()
            .items_center()
            .px(px(16.))
            .py(px(12.))
            .child(
                div()
                    .id("header-title")
                    .test_support()
                    .aria_label(format!("{month} {year}"))
                    .flex_1()
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .text_size(px(24.))
                    .font_weight(FontWeight::BOLD)
                    .child(month)
                    .child(div().text_color(c(n.t3)).font_weight(FontWeight::MEDIUM).child(year)),
            )
            .child(
                div()
                    .id("view-switch")
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
