//! Month view: weeks stream through a virtual list, the header follows the scroll position.
//! The pure parts (which weeks, which month, row packing, "+n more") live in `domain::month`.

mod input;
mod week;

use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::component::{VirtualListScrollHandle, v_virtual_list};
use gpui_kit::{
    AnyElement, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseUpEvent, ParentElement, Pixels,
    Role, SharedString, Size, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, point, px, size,
};

use super::controls::c;
use super::shell::Shell;
use crate::domain::event::Event;
use crate::domain::drag::Drag;
use crate::domain::month::{Plan, SyncState, Weeks, bucket_by_week, cap_for, grid_width};
use crate::theme::resolve::Theme;

/// One week row. Fixed, so a scroll offset maps to a week by division.
pub const WEEK_H: f32 = 128.;
/// A cell: 1px hairline, 6px padding, the 22px date box, 4px gap: the first event row sits here in every cell.
pub(super) const ITEMS_TOP: f32 = 33.;
pub(super) const ITEM_H: f32 = 22.;
/// Weeks kept on each side of the starting week.
const WEEKS_EACH_SIDE: usize = 520;

/// A pointer drag in progress: a new all-day event (`id` None) or a chip being moved.
pub(super) struct MonthDrag {
    pub drag: Drag,
    pub id: Option<u64>,
    /// The cell the pointer was last over: moving within it changes nothing.
    pub over: chrono::NaiveDate,
}

/// What one render draws: the frame's events and, per week, which of them it touches. Built once per
/// render, so the list's callback (once per scroll step) neither clones nor searches the events.
#[derive(Default)]
pub(super) struct Frame {
    pub events: Vec<Event>,
    pub weeks: HashMap<usize, Vec<usize>>,
}

pub struct MonthView {
    pub(super) weeks: Weeks,
    sizes: Rc<Vec<Size<Pixels>>>,
    pub(super) scroll: VirtualListScrollHandle,
    sync: SyncState,
    pub(super) frame: Rc<Frame>,
    pub(super) drag: Option<MonthDrag>,
    /// A chip was pressed: the cell under it starts a move instead of a create.
    pub(super) pending: Option<u64>,
    /// The one event the grid's arrow keys have landed on.
    pub(super) roving: Option<u64>,
    pub(super) focus: FocusHandle,
    pub(super) cap: usize,
}

impl MonthView {
    pub fn new(center: chrono::NaiveDate, ws: chrono::Weekday, cx: &mut gpui_kit::App) -> Self {
        let weeks = Weeks::around(center, ws, WEEKS_EACH_SIDE);
        Self {
            sizes: Rc::new(vec![size(px(1.), px(WEEK_H)); weeks.len()]),
            weeks,
            scroll: VirtualListScrollHandle::new(),
            sync: SyncState::default(),
            frame: Rc::default(),
            drag: None,
            pending: None,
            roving: None,
            focus: cx.focus_handle(),
            cap: 3,
        }
    }
}

impl Shell {
    /// The event the grid's arrow keys are on.
    pub fn month_roving(&self) -> Option<u64> {
        self.month.roving
    }

    fn scrolled(&self) -> f32 {
        f32::from(-self.month.scroll.offset().y)
    }

    /// The list's height once it has been laid out, 0 before.
    fn viewport(&self) -> f32 {
        f32::from(self.month.scroll.bounds().size.height)
    }

    /// Run before the header is built: navigation scrolls the list, scrolling moves the title.
    pub(super) fn month_sync(&mut self) {
        // A changed week start rebuilds the weeks (the header reads them too) and scrolls to the anchor again.
        if self.month.weeks.week_start() != self.model.week_start() {
            self.month.weeks = Weeks::around(self.model.anchor, self.model.week_start(), WEEKS_EACH_SIDE);
            self.month.sync = SyncState::default();
        }
        let top = self.month.weeks.top_week(self.scrolled(), WEEK_H);
        let last_top = (self.viewport() > 0.).then(|| self.month.weeks.last_top(self.viewport(), WEEK_H));
        match self.month.sync.step(self.model.anchor, top, last_top, &self.month.weeks) {
            Plan::Idle => {}
            // Rows have one height, so the week's top edge is a product. (`scroll_to_item` only brings an
            // item into view: it would leave the week at the bottom edge.)
            Plan::Scroll(ix) => self.month.scroll.set_offset(point(px(0.), px(-(ix as f32) * WEEK_H))),
            Plan::Follow(ym) => {
                self.model.follow_month(ym);
                self.month.sync.followed(self.model.anchor);
            }
        }
    }

    /// Frame events: visible calendars only, with an in-progress drag applied so it shows live.
    fn month_events(&self) -> Vec<Event> {
        let mut events: Vec<Event> = self.model.visible_events().into_iter().cloned().collect();
        if let Some(m) = &self.month.drag {
            match m.id {
                Some(id) => events.iter_mut().filter(|e| e.id == id).for_each(|e| *e = m.drag.apply(e)),
                None if m.drag.moved() => {
                    let p = m.drag.preview();
                    let title = super::model::NEW_EVENT_TITLE;
                    events.extend(self.model.new_event_calendar().map(|calendar| Event { id: u64::MAX, title: title.into(), calendar, start: p.start, end: p.end, all_day: true }));
                }
                None => {}
            }
        }
        events
    }

    pub(super) fn month_view(&mut self, t: &Theme, narrow: bool, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let n = &t.neutral;
        self.month.cap = cap_for(grid_width(f32::from(window.viewport_size().width), narrow));
        let events = self.month_events();
        self.month.frame = Rc::new(Frame { weeks: bucket_by_week(&self.month.weeks, &events), events });
        let heads = (0..7).scan(self.month.weeks.week_start(), |d, _| {
            let name = d.to_string();
            *d = d.succ();
            Some(name)
        });
        let heads: Vec<_> = heads
            .enumerate()
            .map(|(i, name)| {
                let head = div().id(SharedString::from(format!("month-head-{i}"))).test_support().role(Role::ColumnHeader).aria_label(name.clone());
                head.flex_1().min_w_0().px(px(12.)).py(px(6.)).flex().justify_end().text_size(px(12.)).font_weight(FontWeight::MEDIUM).text_color(c(n.t3)).child(name)
            })
            .collect();
        let list = v_virtual_list(cx.entity(), "month-list", self.month.sizes.clone(), |this, range, window, cx| this.month_weeks(range, window, cx))
            .track_scroll(&self.month.scroll);
        div()
            .id("month")
            .test_support()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(div().flex().border_t_1().border_color(c(n.line)).children(heads))
            .child(
                div()
                    .id("month-scroll")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    // One Tab stop for the whole grid; the arrow keys move inside it.
                    .track_focus(&self.month.focus.clone().tab_stop(true))
                    .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                        // Modified arrows (Cmd, Ctrl, Alt, Shift, Fn) belong to the window and the system, not the grid.
                        if !e.keystroke.modifiers.modified() && this.month_key(&e.keystroke.key, window, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _: &MouseUpEvent, _, cx| this.cancel_drag(cx)))
                    .child(list),
            )
            .into_any_element()
    }
}
