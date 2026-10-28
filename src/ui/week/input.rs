//! Week and day pointer and keyboard input: press, drag and release go through `geom` hit testing to the
//! domain drag state machine; arrows move the roving event in reading order.

use gpui_kit::{Context, Pixels, Point, Window, point, px};

use super::geom::{GRID_H, Reach, Target, Zone, bar_at, column_of, drag_hit, edge_scroll, hit, scroll_into_view, target_at};
use super::WeekDrag;
use crate::domain::drag::{Drag, Pos};
use crate::domain::event::Event;
use crate::domain::month::{neighbor, reading_order};
use crate::ui::popover::Origin;
use crate::ui::shell::Shell;

impl Shell {
    fn week_event(&self, id: u64) -> Option<Event> {
        self.model.store.events.iter().find(|e| e.id == id).cloned()
    }

    /// The pointer in the frame `geom` measures from, or `None` before the first layout.
    fn week_rel(&self, p: Point<Pixels>) -> Option<(super::geom::Area, f32, f32)> {
        let (area, left, top) = self.week_area()?;
        Some((area, f32::from(p.x) - left, f32::from(p.y) - top))
    }

    /// Press: on a block or bar starts a move (its bottom edge a resize), on empty space a create.
    pub(super) fn week_down(&mut self, p: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.model.screen != crate::ui::model::Screen::Calendar {
            return;
        }
        let Some((area, x, y)) = self.week_rel(p) else { return };
        let frame = self.week.frame.clone();
        let Some(pos) = hit(&area, &frame.days, x, y, false) else { return };
        self.week.pointer = None;
        self.week.drag = None; // ponytail: a stale drag (release lost outside the window) is dropped, not resumed
        self.week.focus.focus(window, cx);
        let (ix, frac) = column_of(&area, frame.days.len(), x);
        let target = match pos {
            Pos::Grid(..) => target_at(&frame.layouts[ix], frac, y - area.all_day_h + area.scroll),
            _ => bar_at(&frame.bars, ix, y).map(|b| Target::Block(b.id, Zone::Body)),
        };
        let drag = match target {
            Some(Target::More) => return,
            Some(Target::Block(id, zone)) => {
                let Some(e) = self.week_event(id) else { return };
                self.week.roving = Some(id);
                let drag = if zone == Zone::Bottom { Drag::resize_bottom(&e, pos) } else { Some(Drag::move_event(&e, pos)) };
                let reach = match (pos, zone) {
                    (Pos::AllDay(_), _) => Reach::Any,
                    (_, Zone::Bottom) => Reach::Grid,
                    _ => Reach::Row,
                };
                drag.map(|drag| WeekDrag { drag, id: Some(id), reach })
            }
            // With every calendar switched off a new event could not be seen: no create drag starts.
            None => {
                let reach = if matches!(pos, Pos::AllDay(_)) { Reach::Any } else { Reach::Grid };
                self.model.new_event_calendar().map(|_| WeekDrag { drag: Drag::create(pos), id: None, reach })
            }
        };
        self.week.drag = drag;
        cx.notify();
    }

    /// What the pointer at `p` is over, for the drag in progress.
    fn week_drag_hit(&self, p: Point<Pixels>, reach: Reach) -> Option<Pos> {
        let (area, x, y) = self.week_rel(p)?;
        drag_hit(&area, &self.week.frame.days, x, y, reach)
    }

    /// Point the drag at what `p` is over.
    fn week_follow(&mut self, p: Point<Pixels>) {
        let Some(reach) = self.week.drag.as_ref().map(|w| w.reach) else { return };
        if let (Some(pos), Some(w)) = (self.week_drag_hit(p, reach), self.week.drag.as_mut()) {
            w.drag.update(pos);
        }
    }

    /// Scroll the grid one step when `p` rests at its top or bottom edge during a real drag (a press that has not
    /// left its 15-minute cell is still a click), then re-read what the pointer is over. `true` if the grid moved.
    pub(super) fn week_edge_step(&mut self, p: Point<Pixels>) -> bool {
        let Some((area, ..)) = self.week_rel(p) else { return false };
        if !self.week.drag.as_ref().is_some_and(|w| w.drag.moved()) {
            return false;
        }
        let step = edge_scroll(f32::from(p.y) - f32::from(self.week.scroll.bounds().origin.y), area.view_h);
        let to = (area.scroll + step).clamp(0., (GRID_H - area.view_h).max(0.));
        if step == 0. || to == area.scroll {
            return false;
        }
        self.week.scroll.set_offset(point(px(0.), px(-to)));
        self.week_follow(p);
        true
    }

    /// Drag: follows the pointer (also outside the grid); at the top and bottom edges the grid scrolls, on moves
    /// here and on every frame (`week_frame`) while the pointer rests.
    pub(super) fn week_move(&mut self, p: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(before) = self.week.drag.as_ref().map(|w| w.drag.preview()) else { return };
        self.week.pointer = Some(p);
        self.week_follow(p);
        let scrolled = self.week_edge_step(p);
        // Only a changed preview (a new 15-minute step or day) redraws: the pointer moves every pixel.
        if scrolled || self.week.drag.as_ref().is_some_and(|w| w.drag.preview() != before) {
            cx.notify();
        }
    }

    /// Release: commit what the drag previewed. A click commits nothing.
    pub(super) fn week_up(&mut self, p: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.week.pointer = None;
        let Some(mut w) = self.week.drag.take() else { return };
        // The release position counts before deciding whether this was a click: no move event may precede it.
        if let Some(pos) = self.week_drag_hit(p, w.reach) {
            w.drag.update(pos);
        }
        if let (Some(id), false) = (w.id, w.drag.moved()) {
            // A press and release on the same block or bar is a click: open it.
            self.open_event(id, Origin::Pointer(p), window, cx);
            return;
        }
        if let Some(pre) = w.drag.end() {
            match w.id {
                Some(id) => {
                    self.model.move_from_drag(id, pre);
                }
                None => self.week.roving = self.model.create_from_drag(pre).or(self.week.roving),
            }
        }
        cx.notify();
    }

    /// Arrows move the one roving event in reading order; Esc cancels a drag. `true` if it acted.
    pub(super) fn week_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let forward = match key {
            "enter" => return self.week.roving.is_some_and(|id| self.model.event(id).is_some() && { self.open_event(id, Origin::Key, window, cx); true }),
            "right" | "down" => true,
            "left" | "up" => false,
            "escape" => {
                self.week.pointer = None;
                let dragging = self.week.drag.take().is_some();
                cx.notify();
                return dragging;
            }
            _ => return false,
        };
        let shown = self.model.shown_days();
        let events: Vec<&Event> = self.model.visible_events().into_iter().filter(|e| e.days().start < shown.end && e.days().end > shown.start).collect();
        let order = reading_order(&events);
        let next = match self.week.roving.filter(|id| order.contains(id)) {
            Some(cur) => neighbor(&order, cur, forward),
            None if forward => order.first().copied(),
            None => order.last().copied(),
        };
        let Some(id) = next else { return false };
        self.week.roving = Some(id);
        if let Some(e) = events.iter().find(|e| e.id == id).filter(|e| !e.all_day) {
            let start = e.start.date().max(shown.start);
            let top = super::geom::minute_to_y(e.segment_on(start).map_or(0, |s| s.0 as i32));
            let h = e.segment_on(start).map_or(0., |s| super::geom::minute_to_y((s.1 - s.0) as i32));
            let (view_h, scroll) = (f32::from(self.week.scroll.bounds().size.height), f32::from(-self.week.scroll.offset().y));
            if view_h > 0. {
                self.week.scroll.set_offset(point(px(0.), px(-scroll_into_view(top, h, scroll, view_h))));
            }
        }
        cx.notify();
        true
    }
}
