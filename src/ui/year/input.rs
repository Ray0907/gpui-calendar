//! Year keyboard input: arrows, Home/End and PageUp/PageDown move the one roving stop (a day or a
//! month title); Enter or Space opens it.

use gpui_kit::Context;

use crate::domain::year::{Cursor, Dir, cursor_in, month_cell, step_cursor};
use crate::ui::shell::Shell;

impl Shell {
    /// `true` if the key was the grid's.
    pub(super) fn year_key(&mut self, key: &str, held: bool, cx: &mut Context<Self>) -> bool {
        let year = self.model.shown_year();
        let cur = cursor_in(self.model.year_cursor, self.model.anchor, year);
        let dir = match key {
            "left" => Dir::Left,
            "right" => Dir::Right,
            "up" => Dir::Up,
            "down" => Dir::Down,
            "home" => Dir::Home,
            "end" => Dir::End,
            "pageup" => Dir::PageUp,
            "pagedown" => Dir::PageDown,
            "enter" | "space" => {
                // A held key repeats: one press opens one view, the repeats are the grid's but do nothing.
                if held {
                    return true;
                }
                match cur {
                    Cursor::Day(d) => self.model.drill_day(d),
                    Cursor::Title(y, m) => self.model.drill_month(y, m),
                }
                self.sync(cx);
                return true;
            }
            _ => return false,
        };
        let next = step_cursor(cur, dir, year, self.model.week_start());
        self.model.year_cursor = Some(next);
        self.year.scroll.scroll_to_item(month_cell(next.month(), self.year.cols).0);
        cx.notify();
        true
    }
}

