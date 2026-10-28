//! Week and day state on the model: which days the grid shows and what the header calls them.

use chrono::Datelike;

use super::{Model, View};
use crate::domain::dates::{DateRange, week_range};

impl Model {
    /// The day columns of the Day and Week views.
    pub fn shown_days(&self) -> DateRange {
        match self.view {
            View::Day => DateRange { start: self.anchor, end: self.anchor + chrono::Duration::days(1) },
            _ => week_range(self.anchor, self.week_start()),
        }
    }

    /// Day view: "Wed 28 Oct" over "2026". Week view: "Oct – Nov" over "2026" when the week crosses a
    /// month, the month name when it does not.
    pub(super) fn week_title(&self) -> (String, String) {
        if self.view == View::Day {
            return (self.anchor.format("%a %-d %b").to_string(), self.anchor.year().to_string());
        }
        let r = self.shown_days();
        let (first, last) = (r.start, r.end - chrono::Duration::days(1));
        let (years, months) = ((first.year(), last.year()), (first.month(), last.month()));
        let year = if years.0 == years.1 { years.0.to_string() } else { format!("{} – {}", years.0, years.1) };
        if months.0 == months.1 {
            (first.format("%B").to_string(), year)
        } else {
            (format!("{} – {}", first.format("%b"), last.format("%b")), year)
        }
    }
}
