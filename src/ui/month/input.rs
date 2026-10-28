//! Month pointer and keyboard input: drag to create across days, drag a chip to move it,
//! arrow keys through the events.

use chrono::NaiveDate;
use gpui_kit::{Context, ScrollStrategy, Window};

use super::{MonthDrag, WEEK_H};
use crate::domain::drag::{Drag, Pos};
use crate::domain::month::{enter, neighbor, reading_order};
use crate::ui::shell::Shell;

impl Shell {
    /// A chip was pressed. The cell under it, reached next, turns this into a move.
    pub(super) fn press_event(&mut self, id: u64) {
        self.month.pending = Some(id);
        self.month.roving = Some(id);
    }

    pub(super) fn press_cell(&mut self, date: NaiveDate, window: &mut Window, cx: &mut Context<Self>) {
        self.month.focus.focus(window, cx);
        let at = Pos::Month(date);
        let moving = self.month.pending.take().and_then(|id| self.model.store.events.iter().find(|e| e.id == id));
        self.month.drag = match moving {
            Some(e) => Some(MonthDrag { id: Some(e.id), drag: Drag::move_event(e, at), over: date }),
            // With every calendar switched off a new event could not be seen: no create drag starts.
            None => self.model.new_event_calendar().map(|_| MonthDrag { id: None, drag: Drag::create(at), over: date }),
        };
        cx.notify();
    }

    pub(super) fn hover_cell(&mut self, date: NaiveDate, cx: &mut Context<Self>) {
        // Every mouse-move lands here: only entering another cell changes the preview.
        if let Some(m) = self.month.drag.as_mut().filter(|m| m.over != date) {
            m.over = date;
            m.drag.update(Pos::Month(date));
            cx.notify();
        }
    }

    /// Release over a cell: commit what the drag previewed. A click commits nothing.
    pub(super) fn release_cell(&mut self, date: NaiveDate, cx: &mut Context<Self>) {
        self.month.pending = None;
        let Some(mut m) = self.month.drag.take() else { return };
        m.drag.update(Pos::Month(date));
        if let Some(p) = m.drag.end() {
            match m.id {
                Some(id) => {
                    self.model.move_from_drag(id, p);
                }
                None => self.month.roving = self.model.create_from_drag(p).or(self.month.roving),
            }
        }
        cx.notify();
    }

    /// Released outside the grid, or Esc: nothing happens to the event.
    pub(super) fn cancel_drag(&mut self, cx: &mut Context<Self>) {
        self.month.pending = None;
        if self.month.drag.take().is_some() {
            cx.notify();
        }
    }

    /// Arrows move the one roving event in reading order; Esc cancels a drag. `true` if it acted.
    pub(super) fn month_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let forward = match key {
            "right" | "down" => true,
            "left" | "up" => false,
            "escape" => {
                let dragging = self.month.drag.is_some();
                self.cancel_drag(cx);
                return dragging;
            }
            _ => return false,
        };
        let events = self.model.visible_events();
        let order = reading_order(&events);
        let next = match self.month.roving.filter(|id| order.contains(id)) {
            Some(cur) => neighbor(&order, cur, forward),
            None => enter(&events, self.model.anchor_month(), forward),
        };
        let Some(id) = next else { return false };
        self.month.roving = Some(id);
        let week = events.iter().find(|e| e.id == id).map(|e| self.month.weeks.index_of(e.start.date()));
        if let Some(ix) = week.filter(|&ix| !self.week_in_view(ix)) {
            self.month.scroll.scroll_to_item(ix, ScrollStrategy::Center);
        }
        cx.notify();
        true
    }

    fn week_in_view(&self, ix: usize) -> bool {
        let viewport = f32::from(self.month.scroll.bounds().size.height);
        self.month.weeks.fully_visible(f32::from(-self.month.scroll.offset().y), viewport, WEEK_H).contains(&ix)
    }
}
