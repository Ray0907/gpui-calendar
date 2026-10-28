//! The event form without a window: text in, a checked `Event` or field errors out, and the edits
//! (add, change, remove) that undo and redo replay. No GPUI import.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use super::dates::format_time;
use super::event::{CalendarId, Event};
use super::layout::SNAP_MINUTES;

/// The form's inputs, in tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Title,
    StartDate,
    StartTime,
    EndDate,
    EndTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormError {
    pub field: Field,
    pub message: &'static str,
}

/// What the fields hold as text. All-day dates are inclusive: `end_date` is the last day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub id: Option<u64>,
    pub title: String,
    pub calendar: CalendarId,
    pub all_day: bool,
    pub start_date: String,
    pub start_time: String,
    pub end_date: String,
    pub end_time: String,
}

/// `2026-10-28`.
pub fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

/// `14:30`, `2:30 PM` or `2:30pm`; minutes always have two digits. In 12-hour mode the AM/PM is required:
/// a bare `2:30` is not guessed to be 02:30.
pub fn parse_time(text: &str, hour24: bool) -> Option<NaiveTime> {
    let text = text.trim().to_ascii_lowercase();
    let (body, pm) = match (text.strip_suffix("am"), text.strip_suffix("pm")) {
        (Some(b), _) => (b.trim_end(), Some(false)),
        (_, Some(b)) => (b.trim_end(), Some(true)),
        _ if !hour24 => return None,
        _ => (text.as_str(), None),
    };
    let (h, m) = body.split_once(':')?;
    if m.len() != 2 || h.is_empty() || h.len() > 2 || !(h.bytes().chain(m.bytes()).all(|b| b.is_ascii_digit())) {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    let h = match pm {
        Some(_) if !(1..=12).contains(&h) => return None,
        Some(pm) => h % 12 + if pm { 12 } else { 0 },
        None => h,
    };
    NaiveTime::from_hms_opt(h, m, 0)
}

fn date_text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

const FIRST_START: (u32, u32) = (9, 0);

fn nine(hour24: bool, plus: u32) -> String {
    format_time(NaiveTime::from_hms_opt(FIRST_START.0 + plus, FIRST_START.1, 0).expect("valid time"), hour24)
}

impl Draft {
    pub fn for_event(e: &Event, hour24: bool) -> Self {
        let (start, end) = (e.start, e.end);
        let (end_date, end_time) = if e.all_day { ((end - Duration::days(1)).date().max(start.date()), nine(hour24, 1)) } else { (end.date(), format_time(end.time(), hour24)) };
        let start_time = if e.all_day { nine(hour24, 0) } else { format_time(start.time(), hour24) };
        Self { id: Some(e.id), title: e.title.clone(), calendar: e.calendar, all_day: e.all_day, start_date: date_text(start.date()), start_time, end_date: date_text(end_date), end_time }
    }

    /// A timed event from `start` snapped down to 15 minutes, one hour long.
    pub fn new_at(start: NaiveDateTime, calendar: CalendarId, hour24: bool) -> Self {
        let snap = SNAP_MINUTES;
        let start = start.date().and_time(NaiveTime::from_hms_opt(start.hour(), start.minute() / snap * snap, 0).expect("valid time"));
        let end = start + Duration::hours(1);
        Self {
            id: None,
            title: String::new(),
            calendar,
            all_day: false,
            start_date: date_text(start.date()),
            start_time: format_time(start.time(), hour24),
            end_date: date_text(end.date()),
            end_time: format_time(end.time(), hour24),
        }
    }

    pub fn new_all_day(day: NaiveDate, calendar: CalendarId) -> Self {
        Self {
            id: None,
            title: String::new(),
            calendar,
            all_day: true,
            start_date: date_text(day),
            start_time: nine(true, 0),
            end_date: date_text(day),
            end_time: nine(true, 1),
        }
    }

    /// Flip the all-day switch. Turning it off leaves times that parse (rewritten in `hour24`) and
    /// puts 09:00 to 10:00 where they do not.
    pub fn set_all_day(&mut self, on: bool, hour24: bool) {
        self.all_day = on;
        if on {
            return;
        }
        for (text, plus) in [(&mut self.start_time, 0), (&mut self.end_time, 1)] {
            // Rewriting what is there: either format is read, as the text may predate `hour24`.
            *text = parse_time(text, true).map_or_else(|| nine(hour24, plus), |t| format_time(t, hour24));
        }
    }

    /// The time format changed: rewrite times that parse, leave the rest for the user.
    pub fn reformat(&mut self, hour24: bool) {
        for text in [&mut self.start_time, &mut self.end_time] {
            if let Some(t) = parse_time(text, true) {
                *text = format_time(t, hour24);
            }
        }
    }

    /// The event this form describes, under `id` when it is a new one, or every error found.
    pub fn check(&self, id: u64, hour24: bool) -> Result<Event, Vec<FormError>> {
        let mut errs = Vec::new();
        let mut bad = |field, message| errs.push(FormError { field, message });
        let title = self.title.trim();
        if title.is_empty() {
            bad(Field::Title, "Enter a title.");
        }
        let start_date = parse_date(&self.start_date);
        let end_date = parse_date(&self.end_date);
        if start_date.is_none() {
            bad(Field::StartDate, "Enter the start date as YYYY-MM-DD.");
        }
        let (start_time, end_time) = (parse_time(&self.start_time, hour24), parse_time(&self.end_time, hour24));
        if !self.all_day && start_time.is_none() {
            bad(Field::StartTime, if hour24 { "Enter the start time, like 14:30." } else { "Enter the start time, like 2:30 PM." });
        }
        if end_date.is_none() {
            bad(Field::EndDate, "Enter the end date as YYYY-MM-DD.");
        }
        if !self.all_day && end_time.is_none() {
            bad(Field::EndTime, if hour24 { "Enter the end time, like 15:30." } else { "Enter the end time, like 3:30 PM." });
        }
        let id = self.id.unwrap_or(id);
        if let (Some(sd), Some(ed)) = (start_date, end_date) {
            if self.all_day {
                if ed < sd {
                    bad(Field::EndDate, "The last day must not be before the first day.");
                } else if errs.is_empty() {
                    return Ok(Event::all_day(id, title, self.calendar, sd, ed));
                }
            } else if let (Some(st), Some(et)) = (start_time, end_time) {
                let (start, end) = (sd.and_time(st), ed.and_time(et));
                if end <= start {
                    bad(Field::EndTime, "The end must be after the start.");
                } else if errs.is_empty() {
                    return Ok(Event { id, title: title.into(), calendar: self.calendar, start, end, all_day: false });
                }
            }
        }
        Err(errs)
    }
}

/// Where the Add button puts a new event: the next whole hour today (the last hour stays on the
/// day), 09:00 on any other day.
pub fn default_start(anchor: NaiveDate, now: NaiveDateTime) -> NaiveDateTime {
    if anchor == now.date() {
        anchor.and_hms_opt((now.hour() + 1).min(23), 0, 0).expect("valid time")
    } else {
        anchor.and_hms_opt(FIRST_START.0, FIRST_START.1, 0).expect("valid time")
    }
}

pub fn next_id(events: &[Event]) -> u64 {
    events.iter().map(|e| e.id).max().map_or(1, |m| m + 1)
}

/// One change to the event list; `apply` does it, `revert` takes it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Add(Event),
    Change { before: Event, after: Event },
    Remove { index: usize, event: Event },
}

impl Edit {
    pub fn apply(&self, events: &mut Vec<Event>) {
        match self {
            Edit::Add(e) => events.push(e.clone()),
            Edit::Change { after, .. } => replace(events, after),
            Edit::Remove { event, .. } => events.retain(|e| e.id != event.id),
        }
    }

    pub fn revert(&self, events: &mut Vec<Event>) {
        match self {
            Edit::Add(e) => events.retain(|x| x.id != e.id),
            Edit::Change { before, .. } => replace(events, before),
            Edit::Remove { index, event } => events.insert((*index).min(events.len()), event.clone()),
        }
    }
}

fn replace(events: &mut [Event], with: &Event) {
    if let Some(e) = events.iter_mut().find(|e| e.id == with.id) {
        *e = with.clone();
    }
}

/// Add `event`, or replace the one with its id.
pub fn save(events: &mut Vec<Event>, event: Event) -> Edit {
    let edit = match events.iter().find(|e| e.id == event.id) {
        Some(before) => Edit::Change { before: before.clone(), after: event },
        None => Edit::Add(event),
    };
    edit.apply(events);
    edit
}

pub fn remove(events: &mut Vec<Event>, id: u64) -> Option<Edit> {
    let index = events.iter().position(|e| e.id == id)?;
    let edit = Edit::Remove { index, event: events[index].clone() };
    edit.apply(events);
    Some(edit)
}

fn day_word(d: NaiveDate, today: NaiveDate) -> String {
    match (d - today).num_days() {
        0 => "Today".into(),
        1 => "Tomorrow".into(),
        -1 => "Yesterday".into(),
        _ => d.format("%a, %b %-d").to_string(),
    }
}

fn length(mins: i64) -> String {
    let plural = |n: i64, unit: &str| format!("{n} {unit}{}", if n == 1 { "" } else { "s" });
    match (mins / 1440, mins / 60, mins % 60) {
        (days, ..) if days > 0 && mins % 1440 == 0 => plural(days, "day"),
        (_, 0, m) => format!("{m} min"),
        (_, h, 0) => plural(h, "hour"),
        (_, h, m) => format!("{h} h {m} min"),
    }
}

/// The well under the dates: ("Tomorrow, 14:00 – 15:00", "1 hour · Wednesday, Oct 28"), or `None`
/// while the dates and times do not parse into an event.
pub fn summary(draft: &Draft, today: NaiveDate, hour24: bool) -> Option<(String, String)> {
    let e = draft.check(0, hour24).ok()?;
    let (first, long) = (e.start.date(), e.start.format("%A, %b %-d").to_string());
    if e.all_day {
        let last = (e.end - Duration::days(1)).date();
        let days = (e.end - e.start).num_minutes();
        let head = if last == first { day_word(first, today) } else { format!("{} – {}", day_word(first, today), day_word(last, today)) };
        return Some((head, format!("All day · {} · {long}", length(days))));
    }
    let (from, to) = (format_time(e.start.time(), hour24), format_time(e.end.time(), hour24));
    let head = if e.end.date() == first {
        format!("{}, {from} – {to}", day_word(first, today))
    } else {
        format!("{}, {from} – {}, {to}", day_word(first, today), day_word(e.end.date(), today))
    };
    Some((head, format!("{} · {long}", length((e.end - e.start).num_minutes()))))
}
