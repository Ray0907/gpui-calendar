//! The mini calendar's month picker: the title opens a card with the year's twelve months (3 by 4) and year arrows. A month jumps the mini
//! month and the view to it; Escape or a click outside closes it and changes nothing.

use chrono::{Datelike, NaiveDate};
use gpui_kit::component::IconName;
use gpui_kit::{
    Anchor, AnyElement, ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Toggled,
    Window, anchored, deferred, div, point, px,
};

use super::controls::{Pointer, c, control, elevation, focus_ring, ghost_hover, icon, ring_mark};
use super::model::{month_cursor, YEARS};
use super::popover::{TrapNext, TrapPrev, anchor_mark};
use super::shell::Shell;
use crate::theme::resolve::Theme;

pub(super) const KEY_CONTEXT: &str = "MiniPick";

/// Card size: 8px padding, a 28px year row, 4px, four 36px rows with 4px between them.
pub const CARD_W: f32 = 220.;
pub const CARD_H: f32 = 2. * PAD + 28. + 4. + 4. * CELL_H + 3. * 4.;
const PAD: f32 = 8.;
const CELL_H: f32 = 36.;
/// Between the title and the card, and the card's smallest distance from the window edge.
const GAP: f32 = 4.;
const MARGIN: f32 = 8.;

pub(super) struct MiniPick {
    pub open: bool,
    /// The month the keyboard is on; its year is the year shown.
    pub cursor: (i32, u32),
    pub title: FocusHandle,
    cells: Vec<FocusHandle>,
    /// The card's tab stops in order (both year arrows, the keyboard's month), written by each render.
    stops: Vec<FocusHandle>,
}

impl MiniPick {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        Self { open: false, cursor: (1970, 1), title: cx.focus_handle(), cells: (0..12).map(|_| cx.focus_handle()).collect(), stops: vec![] }
    }
}

impl Shell {
    /// Whether the month picker is open, for tests.
    pub fn minipick_open(&self) -> bool {
        self.minipick.open
    }

    /// The month the picker's keyboard is on, for tests.
    pub fn minipick_cursor(&self) -> (i32, u32) {
        self.minipick.cursor
    }

    /// The title's focus handle, for tests.
    pub fn minipick_title_focus(&self) -> FocusHandle {
        self.minipick.title.clone()
    }

    /// Whether the root holds the focus, for tests.
    pub fn minipick_root_focused(&self, window: &Window, cx: &gpui_kit::App) -> bool {
        self.root.is_focused(window) || self.root.contains_focused(window, cx)
    }

    fn open_minipick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pop.is_some() {
            return; // the event card owns the window, and its backdrop is below the picker's
        }
        self.minipick.open = true;
        self.minipick.cursor = self.model.mini_ym();
        self.focus_month(window, cx);
        cx.notify();
    }

    /// Close without changing anything; the focus goes back to the title.
    pub(super) fn close_minipick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_minipick(false, window, cx);
    }

    /// The one way the picker closes. The focus goes to the title, or to the root when the rail took the title away.
    pub(super) fn dismiss_minipick(&mut self, rail: bool, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.minipick.open) {
            if rail { self.root.focus(window, cx) } else { self.minipick.title.focus(window, cx) }
            cx.notify();
        }
    }

    fn focus_month(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.minipick.cells[self.minipick.cursor.1 as usize - 1].focus(window, cx);
    }

    fn pick_month(&mut self, year: i32, month: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.close_minipick(window, cx);
        self.model.pick_month(year, month);
        self.sync(cx);
    }

    /// Tab and Shift-Tab stay inside the card: previous year, next year, the keyboard's month, wrapped.
    pub(super) fn minipick_tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.minipick.open {
            return;
        }
        let order = &self.minipick.stops;
        let n = order.len();
        let to = match order.iter().position(|h| h.is_focused(window)) {
            Some(i) if back => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
            None if back => n.saturating_sub(1),
            None => 0,
        };
        if let Some(h) = order.get(to) {
            h.focus(window, cx);
        }
    }

    /// Arrows move through the months, Home and End to the year's ends, Page Up and Page Down through the years. `true` if the key was used.
    /// Only while the focus is in the month grid: on a year arrow these keys are not the card's.
    fn minipick_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.minipick.cells.iter().any(|h| h.is_focused(window)) {
            return false;
        }
        let (year, month) = self.minipick.cursor;
        let next = match key {
            "pageup" => Some(((year - 1).max(*YEARS.start()), month)),
            "pagedown" => Some(((year + 1).min(*YEARS.end()), month)),
            k => month_cursor((year, month), k),
        };
        let Some(next) = next else { return false };
        self.minipick.cursor = next;
        self.focus_month(window, cx);
        cx.notify();
        true
    }

    /// The "October 2026" button above the mini month: it opens and closes the picker.
    pub(super) fn mini_title_button(&mut self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let n = &t.neutral;
        let open = self.minipick.open;
        div()
            .id("mini-title")
            .test_support()
            .role(Role::Button)
            .aria_label("Choose month")
            .aria_expanded(open)
            .track_focus(&self.minipick.title.clone().tab_stop(true))
            .pointer()
            .active(|s| s.opacity(0.85))
            .focus_visible(focus_ring(c(n.surface), c(t.ring), c(n.surface)))
            .flex_1()
            .h(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .whitespace_nowrap()
            .overflow_hidden()
            .hover(ghost_hover(t, n.surface))
            .child(self.model.mini_title())
            .child(anchor_mark(&self.anchors, "mini-title".into()))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                if this.minipick.open {
                    this.close_minipick(window, cx);
                } else {
                    this.open_minipick(window, cx);
                }
            }))
    }

    #[allow(clippy::too_many_arguments)]
    fn year_arrow(&mut self, t: &Theme, id: &'static str, label: &'static str, name: IconName, dir: i32, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        control(id, Role::Button, label, c(t.neutral.surface), c(t.ring), window, cx)
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .hover(ghost_hover(t, t.neutral.surface))
            .child(icon(name, c(t.neutral.t2), 16.))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.minipick.cursor.0 = (this.minipick.cursor.0 + dir).clamp(*YEARS.start(), *YEARS.end()); // the focus stays on the arrow, so Enter steps again
                cx.notify();
            }))
    }

    /// One month of the shown year. Named with its full month and year; the month the view is on is selected (the mini-month day's ring) and
    /// this month is marked as today is (accent fill).
    fn month_cell(&mut self, t: &Theme, year: i32, month: u32, cx: &mut Context<Self>) -> AnyElement {
        let n = &t.neutral;
        let day = NaiveDate::from_ymd_opt(year, month, 1).expect("month 1..=12");
        let selected = (self.model.anchor.year(), self.model.anchor.month()) == (year, month);
        let this_month = (self.model.today.year(), self.model.today.month()) == (year, month);
        let mut b = div()
            .id(SharedString::from(format!("minipick-month-{year}-{month:02}")))
            .test_support()
            .role(Role::Button)
            .aria_label(day.format("%B %Y").to_string())
            .aria_toggled(if selected { Toggled::True } else { Toggled::False })
            .track_focus(&self.minipick.cells[month as usize - 1].clone().tab_stop(self.minipick.cursor.1 == month))
            .pointer()
            .active(|s| s.opacity(0.85))
            .focus_visible(focus_ring(c(n.surface), c(t.ring), if this_month { c(t.accent) } else { c(n.surface) }))
            .w(gpui_kit::relative(1. / 3.))
            .h(px(CELL_H))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .text_size(px(13.))
            .text_color(c(n.t1));
        if this_month {
            b = b.aria_description("current month").bg(c(t.accent)).text_color(c(t.accent_on)).font_weight(FontWeight::SEMIBOLD);
        } else {
            b = b.hover(ghost_hover(t, n.surface));
        }
        if selected {
            // A shadow, not a border, and a fill of the backdrop under it: a shadow paints under a box with no fill.
            b = b.shadow(vec![ring_mark(c(t.ink))]);
            if !this_month {
                b = b.bg(c(n.surface));
            }
        }
        b.child(day.format("%b").to_string())
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.pick_month(year, month, window, cx)))
            .into_any_element()
    }

    /// The open picker, on a backdrop that closes it: under the title, or above it when the window is too short below.
    pub(super) fn minipick_layer(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.minipick.open {
            return None;
        }
        let Some(at) = self.anchors.first("mini-title") else {
            self.minipick.open = false; // no title to hold it
            return None;
        };
        let n = &t.neutral;
        let year = self.minipick.cursor.0;
        let (prev, next) = (self.year_arrow(t, "minipick-prev-year", "Previous year", IconName::ChevronLeft, -1, window, cx), self.year_arrow(t, "minipick-next-year", "Next year", IconName::ChevronRight, 1, window, cx));
        let arrow = |id: &'static str, window: &mut Window, cx: &mut Context<Self>| window.use_keyed_state(id, cx, |_, cx| cx.focus_handle()).read(cx).clone();
        self.minipick.stops = vec![arrow("minipick-prev-year", window, cx), arrow("minipick-next-year", window, cx), self.minipick.cells[self.minipick.cursor.1 as usize - 1].clone()];
        let cells: Vec<_> = (1..=12).map(|m| self.month_cell(t, year, m, cx)).collect();
        let card = div()
            .id("minipick")
            .test_support()
            .role(Role::Group)
            .aria_label("Choose month")
            .key_context(KEY_CONTEXT)
            .on_action(cx.listener(|this, _: &TrapNext, window, cx| this.minipick_tab(false, window, cx)))
            .on_action(cx.listener(|this, _: &TrapPrev, window, cx| this.minipick_tab(true, window, cx)))
            .occlude() // a click on the card is not a click on the backdrop behind it
            .w(px(CARD_W))
            .h(px(CARD_H))
            .p(px(PAD))
            .rounded(px(12.))
            .border_1()
            .border_color(c(n.border))
            .bg(c(n.surface))
            .shadow(elevation(t))
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                if !e.keystroke.modifiers.modified() && this.minipick_key(&e.keystroke.key, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb(px(4.))
                    .child(prev)
                    .child(div().id("minipick-year").test_support().aria_label(format!("{year}")).text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(format!("{year}")))
                    .child(next),
            )
            .child(div().flex().flex_wrap().gap_y(px(4.)).children(cells));
        let (view_w, view_h) = (f32::from(window.viewport_size().width), f32::from(window.viewport_size().height));
        // Not popover/place.rs: its `side` picks left or right of a trigger, the picker sits above or below its title, centred.
        let below = view_h - f32::from(at.bottom()) >= CARD_H + GAP + MARGIN;
        let x = (f32::from(at.center().x) - CARD_W / 2.).clamp(MARGIN, (view_w - CARD_W - MARGIN).max(MARGIN));
        let (y, corner) = if below { (at.bottom() + px(GAP), Anchor::TopLeft) } else { (at.top() - px(GAP), Anchor::BottomLeft) };
        let backdrop = div().id("minipick-backdrop").test_support().absolute().inset_0().occlude().on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close_minipick(window, cx)));
        let placed = anchored().position(point(px(x), y)).anchor(corner).snap_to_window_with_margin(px(MARGIN)).child(card);
        Some(deferred(div().absolute().inset_0().child(backdrop).child(placed)).with_priority(3).into_any_element())
    }
}
