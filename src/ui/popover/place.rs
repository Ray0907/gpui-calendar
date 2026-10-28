//! Which side of its trigger the popover opens on. Vertical fit is the `anchored` element's job
//! (it slides the card back inside the window); the side decides before that, because mirroring around
//! a point would cover the trigger. `Anchors` holds where the triggers were painted.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::{Bounds, IntoElement, Pixels, Styled, canvas, point, px, size};

pub const WIDTH: f32 = 720.;
pub const GAP: f32 = 8.;
pub const MARGIN: f32 = 8.;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    /// The card's left edge is at `x`.
    Right(f32),
    /// The card's right edge is at `x`.
    Left(f32),
}

/// The card's width: `WIDTH`, or the window less a `MARGIN` each side when that is narrower.
pub fn card_width(window: f32) -> f32 {
    WIDTH.min(window - 2. * MARGIN).max(0.)
}

/// `anchor` is the trigger's `(left, right)`; `window` is the window's width.
pub fn side((left, right): (f32, f32), window: f32) -> Side {
    let width = card_width(window);
    if right + GAP + width + MARGIN <= window {
        Side::Right(right + GAP)
    } else if left - GAP - width >= MARGIN {
        Side::Left(left - GAP)
    } else {
        // Neither side has room (a wide trigger in a narrow window): overlap it, centered.
        Side::Right(((window - width) / 2.).max(MARGIN))
    }
}

/// Where each trigger was last painted, by key (`ev-<id>` once per piece of an event, `add`, `banner`).
/// Recording is idempotent: the same bounds twice in a frame are one piece.
#[derive(Clone, Default)]
pub(in crate::ui) struct Anchors(Rc<RefCell<HashMap<String, Vec<Bounds<Pixels>>>>>);

impl Anchors {
    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    pub fn record(&self, key: &str, b: Bounds<Pixels>) {
        let mut all = self.0.borrow_mut();
        let pieces = all.entry(key.to_string()).or_default();
        if !pieces.contains(&b) {
            pieces.push(b);
        }
    }

    pub fn pieces(&self, key: &str) -> Vec<Bounds<Pixels>> {
        self.0.borrow().get(key).cloned().unwrap_or_default()
    }

    /// The first piece painted under `key`, without copying the rest.
    pub fn first(&self, key: &str) -> Option<Bounds<Pixels>> {
        self.0.borrow().get(key).and_then(|v| v.first()).copied()
    }
}

/// Where a card with no trigger is centered: the main card below its header, kept `MARGIN` off the bottom.
pub fn below_header(main: Bounds<Pixels>) -> Bounds<Pixels> {
    let head = px(crate::ui::header::HEADER_H);
    Bounds::new(point(main.left(), main.top() + head), size(main.size.width, (main.size.height - head - px(MARGIN)).max(px(0.))))
}

/// An invisible box over its parent that records where the parent was painted under `key`.
pub(in crate::ui) fn anchor_mark(anchors: &Anchors, key: String) -> impl IntoElement {
    let anchors = anchors.clone();
    canvas(move |b, _, _| anchors.record(&key, b), |_, _, _, _| ()).absolute().inset_0()
}

/// Painted first in the window: forgets last frame's anchors before this frame's marks record theirs.
pub(in crate::ui) fn anchor_reset(anchors: &Anchors) -> impl IntoElement {
    let anchors = anchors.clone();
    canvas(move |_, _, _| anchors.clear(), |_, _, _, _| ()).absolute().size_0()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_the_same_bounds_twice_keeps_one_piece() {
        let (a, b) = (Bounds::new(point(px(0.), px(0.)), size(px(10.), px(10.))), Bounds::new(point(px(0.), px(20.)), size(px(10.), px(10.))));
        let anchors = Anchors::default();
        for bounds in [a, a, b, a] {
            anchors.record("ev-1", bounds);
        }
        assert_eq!(anchors.pieces("ev-1"), [a, b], "once per piece, in paint order");
        anchors.clear();
        assert!(anchors.pieces("ev-1").is_empty());
    }

    #[test]
    fn opens_to_the_right_when_it_fits() {
        assert_eq!(side((100., 200.), 1440.), Side::Right(208.));
    }

    #[test]
    fn flips_left_at_the_right_edge() {
        assert_eq!(side((1000., 1100.), 1200.), Side::Left(992.));
        assert_eq!(side((100., 464.), 1200.), Side::Right(472.), "464 + 8 + 720 + 8 = 1200 still fits");
        assert_eq!(side((736., 1000.), 1200.), Side::Left(728.), "728 - 720 = 8 still clears the margin");
        assert_eq!(side((735., 1000.), 1200.), Side::Right(240.), "no side fits: centered");
    }

    #[test]
    fn the_card_shrinks_to_fit_a_narrow_window() {
        assert_eq!(card_width(1440.), WIDTH);
        assert_eq!(card_width(736.), WIDTH, "720 + 2 * 8 just fits");
        assert_eq!(card_width(480.), 464.);
        assert_eq!(card_width(10.), 0., "never negative");
        for window in [300., 480., 735., 900.] {
            let w = card_width(window);
            let left = match side((100., 200.), window) {
                Side::Right(x) => x,
                Side::Left(x) => x - w,
            };
            assert!(left >= MARGIN && left + w <= window - MARGIN, "{window}: card {left}..{} overflows", left + w);
        }
    }

    #[test]
    fn centers_when_neither_side_fits() {
        assert_eq!(side((100., 800.), 1200.), Side::Right(240.));
        assert_eq!(side((0., 300.), 380.), Side::Right(MARGIN));
    }
}
