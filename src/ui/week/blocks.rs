//! Event blocks on the time grid, all-day bars, and the "+n more" block.

use chrono::NaiveDate;
use gpui_kit::{
    AnyElement, CursorStyle, FontWeight, InteractiveElement, IntoElement, ParentElement, Role, SharedString, Styled, TestSupportExt, div, px, relative,
    prelude::*,
};

use super::Frame;
use super::geom::{Bar, Block, EDGE_PX, LANE_H, MORE_X, More, block_name, block_range, event_name, lane_top};
use super::Shell;
use crate::domain::event::Event;
use crate::theme::resolve::Theme;
use crate::ui::popover::anchor_mark;
use crate::ui::controls::{c, elevation, ghost_hover, hover_wash, ring, shape, shape_of};

/// Blocks at least this tall show the time under the title.
const TWO_LINES: f32 = 34.;

impl Shell {
    /// A timed event's piece on `day`: tint, text color, 3px bar at the left, calendar shape in the top-right corner.
    /// `(focused, lifted, hour24)`: the roving ring, the shadow of a pressed block, the time format.
    pub(super) fn week_block(&self, t: &Theme, e: &Event, b: &Block, day: NaiveDate, (focused, lifted, hour24): (bool, bool, bool)) -> AnyElement {
        let Some(cal) = t.cal(e.calendar) else { return div().into_any_element() };
        let name = block_name(e, day, self.model.calendar_name(e.calendar), hour24);
        let range = block_range(e, day, hour24);
        let inner = div()
            .id(SharedString::from(format!("week-ev-{}-{day}", e.id)))
            .test_support()
            .role(Role::Button)
            .aria_label(name)
            .relative()
            .size_full()
            .overflow_hidden()
            .rounded(px(6.))
            .bg(c(cal.tint))
            .hover(hover_wash(t.neutral.t1, cal.tint))
            // A held block dims to 85% (its ring would show through the dim, so it drops); once it moves it lifts at full opacity.
            .when(!lifted, |s| s.active(|s| s.opacity(0.85).shadow(vec![])))
            .text_color(c(cal.text))
            .text_size(px(12.))
            .line_height(px(16.))
            .pl(px(10.))
            .pr(px(18.))
            .py(px(2.))
            .cursor(if lifted { CursorStyle::ClosedHand } else { CursorStyle::OpenHand })
            .when(lifted, |s| s.shadow(elevation(t)))
            .when(focused, |s| s.shadow(ring(c(t.neutral.surface), c(t.ring))))
            .child(anchor_mark(&self.anchors, format!("ev-{}", e.id)))
            .child(div().absolute().left(px(3.)).top(px(4.)).bottom(px(4.)).w(px(3.)).rounded(px(3.)).bg(c(cal.text)))
            .child(div().absolute().top(px(6.)).right(px(6.)).child(shape(shape_of(e.calendar), c(cal.dot))))
            .child(div().truncate().font_weight(FontWeight::MEDIUM).child(e.title.clone()))
            .when(b.h >= TWO_LINES, |s| s.child(div().truncate().opacity(0.8).child(range)))
            // The bottom edge of the piece that ends the event: a vertical-resize cursor (no handler; the grid's own pointer logic resizes).
            .when(b.ends, |s| s.child(div().absolute().left_0().right_0().bottom_0().h(px(EDGE_PX.min(b.h / 2.))).cursor(CursorStyle::ResizeUpDown)));
        let (lo, hi) = b.x();
        self.slot(b.top, b.h, lo, hi - lo).child(inner).into_any_element()
    }

    /// An absolutely placed box at `top`, `left..left + width` as fractions of the column, with 3px beside it.
    fn slot(&self, top: f32, h: f32, left: f32, width: f32) -> gpui_kit::Div {
        div().absolute().top(px(top + 1.)).h(px(h - 2.)).left(relative(left)).w(relative(width)).px(px(3.))
    }

    /// Overlaps past the second column: a count in the strip the cluster leaves free, at the first hidden event's start.
    pub(super) fn week_more(&self, t: &Theme, m: &More, day: NaiveDate, focused: bool) -> AnyElement {
        let text = format!("+{} more", m.n);
        let n = &t.neutral;
        let chip = div()
            .id(SharedString::from(format!("week-more-{day}-{}", m.top as i32)))
            .test_support()
            .role(Role::Button)
            .aria_label(text.clone())
            .size_full()
            .px(px(4.))
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .rounded(px(6.))
            .bg(c(n.surface))
            .hover(ghost_hover(t, n.surface))
            .active(|s| s.opacity(0.85))
            .border_1()
            .border_color(c(n.line))
            .text_size(px(12.))
            .text_color(c(n.t2))
            .when(focused, |s| s.shadow(ring(c(n.surface), c(t.ring))))
            .child(format!("+{}", m.n)); // the strip is narrow: the name says "more", the chip shows the count
        self.slot(m.top, m.h, MORE_X.0, MORE_X.1 - MORE_X.0).child(chip).into_any_element()
    }

    /// An all-day event as a pill over its day columns; a cut end is square.
    pub(super) fn week_bar(&self, t: &Theme, frame: &Frame, b: &Bar, roving: Option<u64>, hour24: bool) -> Option<AnyElement> {
        let e = frame.event(b.id)?;
        let cal = t.cal(e.calendar)?;
        let n = frame.days.len() as f32;
        let lifted = self.week.drag.as_ref().is_some_and(|w| w.id == Some(e.id) && w.drag.moved());
        let pill = div()
            .id(SharedString::from(format!("week-ad-{}", e.id)))
            .test_support()
            .role(Role::Button)
            .aria_label(event_name(e, self.model.calendar_name(e.calendar), hour24))
            .size_full()
            .flex()
            .items_center()
            .gap(px(6.))
            .px(px(8.))
            .rounded(px(6.))
            .when(b.from_prev, |s| s.rounded_tl(px(0.)).rounded_bl(px(0.)))
            .when(b.to_next, |s| s.rounded_tr(px(0.)).rounded_br(px(0.)))
            .bg(c(cal.tint))
            .hover(hover_wash(t.neutral.t1, cal.tint))
            .when(!lifted, |s| s.active(|s| s.opacity(0.85).shadow(vec![])))
            .text_color(c(cal.text))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .cursor(if lifted { CursorStyle::ClosedHand } else { CursorStyle::OpenHand })
            .when(lifted, |s| s.shadow(elevation(t)))
            .when(roving == Some(e.id), |s| s.shadow(ring(c(t.neutral.surface), c(t.ring))))
            .child(anchor_mark(&self.anchors, format!("ev-{}", e.id)))
            .child(shape(shape_of(e.calendar), c(cal.dot)))
            .child(div().flex_1().min_w_0().truncate().child(e.title.clone()));
        Some(div().absolute().top(px(lane_top(b.lane))).h(px(LANE_H - 2.)).left(relative(b.col as f32 / n)).w(relative(b.span as f32 / n)).px(px(2.)).child(pill).into_any_element())
    }
}
