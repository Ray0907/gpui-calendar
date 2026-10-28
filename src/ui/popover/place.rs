//! Which side of its trigger the popover opens on. Vertical fit is the `anchored` element's job
//! (it slides the card back inside the window); the side decides before that, because mirroring around
//! a point would cover the trigger. `Anchors` holds where the triggers were painted.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui_kit::{Bounds, IntoElement, Pixels, Styled, canvas};

pub const WIDTH: f32 = 400.;
pub const GAP: f32 = 8.;
pub const MARGIN: f32 = 8.;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    /// The card's left edge is at `x`.
    Right(f32),
    /// The card's right edge is at `x`.
    Left(f32),
}

/// `anchor` is the trigger's `(left, right)`; `window` is the window's width.
pub fn side((left, right): (f32, f32), window: f32) -> Side {
    if right + GAP + WIDTH + MARGIN <= window {
        Side::Right(right + GAP)
    } else if left - GAP - WIDTH >= MARGIN {
        Side::Left(left - GAP)
    } else {
        // Neither side has room (a wide trigger in a narrow window): overlap it, centered.
        Side::Right(((window - WIDTH) / 2.).max(MARGIN))
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
    use gpui_kit::{point, px, size};

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
        assert_eq!(side((1200., 1400.), 1440.), Side::Left(1192.));
        assert_eq!(side((1000., 1024.), 1440.), Side::Right(1032.), "1032 + 400 + 8 = 1440 still fits");
        assert_eq!(side((1000., 1025.), 1440.), Side::Left(992.));
    }

    #[test]
    fn centers_when_neither_side_fits() {
        assert_eq!(side((100., 800.), 960.), Side::Right(280.));
        assert_eq!(side((0., 300.), 380.), Side::Right(MARGIN));
    }
}
