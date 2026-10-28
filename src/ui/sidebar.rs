//! Sidebar card (236 wide) and the 64px rail it collapses to below 1120.

use gpui_kit::component::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Role, Styled, Toggled, Window, div,
    linear_color_stop, linear_gradient, px,
};
use gpui_kit::{StatefulInteractiveElement, TestSupportExt, prelude::*};

use super::controls::{banner_scrim, banner_wash, c, control, elevation, eyebrow, icon, on_color, ring_mark, shape, shape_of};
use super::model::MiniDay;
use super::shell::Shell;
use crate::store::Calendar;
use crate::theme::resolve::Theme;

impl Shell {
    pub(super) fn sidebar(&mut self, t: &Theme, narrow: bool, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let card = div()
            .id("sidebar")
            .test_support()
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .p(px(8.))
            .rounded(px(14.))
            .bg(c(t.neutral.surface))
            .shadow(elevation(t));
        if narrow {
            let (add, toggles) = (self.add_button(t, window, cx), self.calendar_toggles(t, false, window, cx));
            let account = self.account_button(t, window, cx);
            return card.w(px(64.)).items_center().gap(px(4.)).child(add).children(toggles).child(div().flex_1()).child(account).into_any_element();
        }
        let banner = self.banner(t, window, cx);
        let rows = self.calendar_toggles(t, true, window, cx);
        let mini = self.mini_month(t, window, cx);
        let user = self.user_row(t, window, cx);
        let add = self.add_button(t, window, cx);
        card.w(px(236.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .pt(px(4.))
                    .px(px(8.))
                    .pb(px(12.))
                    .child(div().size(px(22.)).rounded(px(6.)).bg(c(t.ink)))
                    .child(div().flex_1().text_size(px(16.)).font_weight(FontWeight::BOLD).child("Calendar"))
                    .child(add),
            )
            .children(banner)
            .child(div().mt(px(4.)).mx(px(8.)).mb(px(6.)).child(eyebrow(t, "Calendars")))
            // The one region that gives way when the window is short, so the mini month and the
            // Appearance entry below stay in the card.
            .child(div().id("calendar-list").test_support().flex_1().min_h(px(36.)).overflow_y_scroll().children(rows))
            .child(mini)
            .child(user)
            .into_any_element()
    }

    fn add_button(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        control("add-event", Role::Button, "New event", c(t.neutral.surface), c(t.ring), window, cx)
            .size(px(40.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(c(t.ink))
            .child(icon(IconName::Plus, c(t.on_ink), 16.))
            // Block 8 opens the event form; until then the button only acknowledges the press.
            .on_click(cx.listener(|_, _: &ClickEvent, _, _| {}))
    }

    fn banner(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let n = self.model.next_up()?;
        let b = t.banner;
        let button = control("open-event", Role::Button, n.label(), c(b.from), on_color(), window, cx)
            .mt(px(10.))
            .h(px(32.))
            .flex()
            .items_center()
            .gap(px(6.))
            .pl(px(14.))
            .pr(px(6.))
            .w_auto()
            .rounded_full()
            .bg(banner_scrim())
            .text_size(px(12.))
            .text_color(on_color())
            .child(n.label())
            .child(
                div()
                    .size(px(20.))
                    .rounded_full()
                    .bg(banner_wash())
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(IconName::ChevronRight, on_color(), 12.)),
            )
            .on_click(cx.listener(|_, _: &ClickEvent, _, _| {}));
        Some(
            div()
                .id("banner")
                .test_support()
                .mb(px(14.))
                .px(px(14.))
                .py(px(12.))
                .rounded(px(6.))
                .bg(linear_gradient(90., linear_color_stop(c(b.from), 0.), linear_color_stop(c(b.to), 1.)))
                .child(div().text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(c(b.label)).child("NEXT UP"))
                .child(
                    div()
                        .mt(px(3.))
                        .text_size(px(18.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(c(b.value))
                        .child(n.title.clone()),
                )
                .child(div().text_size(px(12.)).text_color(c(b.secondary)).child(format!("{} · {}", n.relative, n.clock)))
                .child(div().flex().child(button)),
        )
    }

    /// One checkbox control per calendar: `full` rows carry the name, rail toggles carry a tooltip.
    fn calendar_toggle(&mut self, t: &Theme, cal: &Calendar, full: bool, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let id = cal.id;
        let (dot, check) = t.cal(id).map_or((c(t.ink), c(t.on_ink)), |k| (c(k.dot), c(k.check)));
        let name = cal.name.clone();
        let box_ = div()
            .size(px(16.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .border_1()
            .border_color(if cal.visible { dot } else { c(t.neutral.border) })
            .when(cal.visible, |s| s.bg(dot))
            .children(cal.visible.then(|| icon(IconName::Check, check, 12.)));
        let tip = name.clone();
        control(format!("calendar-row-{id}"), Role::CheckBox, name.clone(), c(t.neutral.surface), c(t.ring), window, cx)
            .h(px(if full { 36. } else { 40. }))
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(8.))
            .rounded(px(6.))
            .hover(|s| s.bg(c(t.neutral.page)))
            .aria_toggled(if cal.visible { Toggled::True } else { Toggled::False })
            .child(box_)
            .child(shape(shape_of(id), dot))
            .children(full.then(|| div().text_size(px(13.)).child(name)))
            .when(!full, move |this| this.tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx)))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.model.toggle_calendar(id);
                this.sync(cx);
            }))
    }

    /// The calendar toggles: full rows in the sidebar, icon-only toggles in the rail.
    fn calendar_toggles(&mut self, t: &Theme, full: bool, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let cals = self.model.store.calendars.clone();
        cals.iter().map(|k| self.calendar_toggle(t, k, full, window, cx).into_any_element()).collect()
    }

    fn mini_month(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let n = &t.neutral;
        let prev = self.mini_nav(t, "mini-prev", "Previous month in mini calendar", IconName::ChevronLeft, -1, window, cx);
        let next = self.mini_nav(t, "mini-next", "Next month in mini calendar", IconName::ChevronRight, 1, window, cx);
        let letters = self.model.mini_weekdays();
        let days = self.model.mini_days();
        let cells: Vec<_> = days.iter().map(|d| self.mini_day(t, d, window, cx)).collect();
        div()
            .id("mini-month")
            .test_support()
            .px(px(8.))
            .py(px(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(6.))
                    .text_size(px(12.))
                    .child(self.model.mini_title())
                    .child(div().flex().child(prev).child(next)),
            )
            .child(div().flex().children(letters.into_iter().map(|l| {
                div()
                    .w(px(28.))
                    .flex_1()
                    .flex()
                    .justify_center()
                    .h(px(24.))
                    .items_center()
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(n.t3))
                    .child(l)
            })))
            .child(div().flex().flex_wrap().gap_y(px(2.)).children(cells))
    }

    #[allow(clippy::too_many_arguments)]
    fn mini_nav(&mut self, t: &Theme, id: &'static str, label: &'static str, name: IconName, dir: i32, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        control(id, Role::Button, label, c(t.neutral.surface), c(t.ring), window, cx)
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .child(icon(name, c(t.neutral.t2), 16.))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.model.mini_step(dir);
                this.sync(cx);
            }))
    }

    fn mini_day(&mut self, t: &Theme, d: &MiniDay, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let date = d.date;
        let n = &t.neutral;
        let mut b = control(format!("mini-day-{date}"), Role::Button, d.name.clone(), c(n.surface), c(t.ring), window, cx)
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .text_size(px(12.))
            .text_color(if d.out_of_month { c(n.t3) } else { c(n.t1) })
            .aria_toggled(if d.selected { Toggled::True } else { Toggled::False });
        // Both marks can apply to one day: the accent circle (today) and the ink ring (selected).
        if d.accent_fill() {
            b = b.aria_description("current date").bg(c(t.accent)).text_color(c(t.accent_on)).font_weight(FontWeight::SEMIBOLD);
        }
        if d.ring() {
            b = b.shadow(vec![ring_mark(c(t.ink))]); // a shadow, not a border: the cell keeps its size
        }
        div().w(gpui_kit::relative(1. / 7.)).flex().justify_center().child(b.child(date.format("%-d").to_string()).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.model.pick_day(date);
                this.sync(cx);
            },
        )))
    }

    /// Opens and closes the Appearance screen (pressed while it is open); in the rail it is the only way there.
    fn account_button(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let n = &t.neutral;
        let open = self.model.screen == super::model::Screen::Appearance;
        control("account-menu", Role::Button, "Appearance", c(n.surface), c(t.ring), window, cx)
            .size(px(40.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .aria_toggled(if open { Toggled::True } else { Toggled::False })
            .child(icon(IconName::ChevronsUpDown, c(n.t2), 16.))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.model.toggle_appearance();
                this.sync(cx);
            }))
    }

    fn user_row(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let n = &t.neutral;
        let account = self.account_button(t, window, cx);
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .pt(px(8.))
            .px(px(8.))
            .pb(px(2.))
            .mt(px(6.))
            .border_t_1()
            .border_color(c(n.line))
            .text_color(c(n.t2))
            .child(div().size(px(22.)).rounded_full().bg(c(n.line)))
            .child(div().flex_1().child("Alex Chen"))
            .child(account)
    }
}
