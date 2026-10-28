//! Year view state on the model: drilling into a day or a month.

use chrono::{Datelike, NaiveDate, Weekday};

use super::{Model, View, month_start};

impl Model {
    /// The year the Year view shows.
    pub fn shown_year(&self) -> i32 {
        self.anchor.year()
    }

    /// A day was chosen in the Year view: show it in the Day view.
    pub fn drill_day(&mut self, date: NaiveDate) {
        self.focus_anchor(date);
        self.set_view(View::Day);
    }

    /// A month title was chosen in the Year view: show that month in the Month view.
    pub fn drill_month(&mut self, year: i32, month: u32) {
        self.focus_anchor(month_start(NaiveDate::from_ymd_opt(year, month, 1).expect("valid year/month")));
        self.set_view(View::Month);
    }

    /// Drop the roving stop and ask the Year view to scroll back to the anchor, even if it did not move.
    /// Every change of what the grid shows goes through here.
    pub(super) fn reset_year_cursor(&mut self) {
        self.year_cursor = None;
        self.year_epoch = self.year_epoch.wrapping_add(1);
    }

    pub fn year_epoch(&self) -> u32 {
        self.year_epoch
    }

    /// The month grids change shape with the first weekday, so the stop is dropped.
    pub fn set_week_start(&mut self, ws: Weekday) {
        self.store.settings.week_start = ws;
        self.reset_year_cursor();
        self.persist();
    }
}
