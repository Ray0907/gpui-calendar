//! The event popover: open from a click or Enter on an event, from the Add button and from the banner;
//! edit, save, delete (with undo) or cancel. The text rules are in `domain::form`; `place` picks the
//! side; `view` draws it.

mod date;
mod fields;
mod place;
mod time;
mod view;

pub(super) use place::{Anchors, anchor_mark, anchor_reset};

use std::cell::RefCell;
use std::time::Duration;

use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::{App, AppContext, Bounds, Context, Entity, FocusHandle, Pixels, Point, Subscription, Window};

use self::date::Dates;
use self::time::Times;
use super::shell::Shell;
use crate::domain::form::{Draft, Field, FormError};

const TOAST_SECS: u64 = 8;
pub(super) const KEY_CONTEXT: &str = "EventPopover";

gpui_kit::actions!(event_popover, [TrapNext, TrapPrev]);

/// Bind Tab and Shift-Tab inside the popover (once per app).
pub(super) fn bind_keys(cx: &mut App) {
    cx.bind_keys([gpui_kit::KeyBinding::new("tab", TrapNext, Some(KEY_CONTEXT)), gpui_kit::KeyBinding::new("shift-tab", TrapPrev, Some(KEY_CONTEXT))]);
}

/// How the popover was asked for. A keyboard open appears at once, with no motion.
#[derive(Clone, Copy)]
pub(super) enum Origin {
    Pointer(Point<Pixels>),
    Key,
}

struct Inputs {
    title: Entity<InputState>,
    notes: Entity<TextareaState>,
}

impl Inputs {
    /// The typed field; the days and times are picked, so `None` for them.
    fn of(&self, f: Field) -> Option<&Entity<InputState>> {
        (f == Field::Title).then_some(&self.title)
    }
}

/// The open popover.
pub(super) struct Form {
    /// Calendar, all-day and id are live here. The text fields, notes included, live only in `inputs`
    /// (`draft`'s copies are stale or empty after open): always go through `read`.
    draft: Draft,
    errors: Vec<FormError>,
    inputs: Inputs,
    dates: Dates,
    times: Times,
    anchor: Option<Bounds<Pixels>>,
    by_key: bool,
    /// Counts opens, so each pointer open fades in once.
    seq: usize,
    trap: FocusHandle,
    /// Every tab stop in the card in reading order; written by each render.
    order: RefCell<Vec<FocusHandle>>,
    /// The All day control's handle, written by each render: turning All day on moves the focus here out of a time field it disables.
    all_day: RefCell<Option<FocusHandle>>,
    return_to: Option<FocusHandle>,
    _subs: Vec<Subscription>,
}

impl Form {
    /// The draft with the text as typed.
    fn read(&self, cx: &App) -> Draft {
        let text = |i: &Entity<InputState>| i.read(cx).value().to_string();
        Draft {
            title: text(&self.inputs.title),
            start_date: self.dates.text(Field::StartDate, cx),
            start_time: self.times.text(Field::StartTime, cx),
            end_date: self.dates.text(Field::EndDate, cx),
            end_time: self.times.text(Field::EndTime, cx),
            notes: self.inputs.notes.read(cx).value().to_string(),
            ..self.draft.clone()
        }
    }

    fn error(&self, f: Field) -> Option<&'static str> {
        self.errors.iter().find(|e| e.field == f).map(|e| e.message)
    }
}

impl Shell {
    /// `Some(true)` when the open popover was asked for with the keyboard (and so did not animate).
    pub fn popover_by_key(&self) -> Option<bool> {
        self.pop.as_ref().map(|f| f.by_key)
    }

    /// What the popover's fields hold now.
    pub fn popover_draft(&self, cx: &App) -> Option<Draft> {
        self.pop.as_ref().map(|f| f.read(cx))
    }

    pub fn popover_errors(&self) -> Vec<FormError> {
        self.pop.as_ref().map(|f| f.errors.clone()).unwrap_or_default()
    }

    /// Replace the text of the title, or the time of a time field, as typing or picking would (the days are picked, not typed).
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn popover_set(&mut self, field: Field, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(f) = self.pop.as_mut() else { return };
        if let Some(input) = f.inputs.of(field).cloned() {
            input.update(cx, |s, cx| s.set_value(text.to_string(), window, cx));
        } else if f.times.of(field).is_some() {
            f.times.set(field, text, window, cx);
            f.errors.retain(|e| e.field != field);
        }
        cx.notify();
    }

    /// Open the form on `draft` as a keyboard open would, text and all (the days too, readable or not).
    pub fn show_draft(&mut self, draft: Draft, window: &mut Window, cx: &mut Context<Self>) {
        self.open_form(draft, None, Origin::Key, window, cx);
    }

    /// Flip All day, as its control does but without the click that would move the focus.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn popover_toggle_all_day(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_form_all_day(window, cx);
    }

    /// Replace the notes, as typing would.
    pub fn popover_set_notes(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = self.pop.as_ref() {
            f.inputs.notes.update(cx, |s, cx| s.set_value(text.to_string(), window, cx));
            cx.notify();
        }
    }

    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_deref()
    }

    /// The focus handle of the open popover's card.
    pub fn popover_focus(&self) -> Option<FocusHandle> {
        self.pop.as_ref().map(|f| f.trap.clone())
    }

    /// The grid that has focus in the current view: where focus goes when nothing else asked for it.
    pub fn grid_focus(&self) -> FocusHandle {
        use super::model::View;
        match self.model.view {
            View::Month => self.month.focus.clone(),
            View::Day | View::Week => self.week.focus.clone(),
            View::Year => self.year.focus.clone(),
        }
    }

    fn anchor_for(&self, key: &str, from: Origin) -> Option<Bounds<Pixels>> {
        let pieces = self.anchors.pieces(key);
        match from {
            Origin::Pointer(p) => pieces.iter().find(|b| b.contains(&p)).or(pieces.first()).copied(),
            Origin::Key => pieces.first().copied(),
        }
    }

    pub(super) fn open_event(&mut self, id: u64, from: Origin, window: &mut Window, cx: &mut Context<Self>) {
        let Some(e) = self.model.event(id) else { return };
        let draft = Draft::for_event(e, self.model.store.settings.time_format.hour24());
        let anchor = self.anchor_for(&format!("ev-{id}"), from);
        self.open_form(draft, anchor, from, window, cx);
    }

    /// Open event `id` as a keyboard open would: at once, anchored to its first painted piece.
    pub fn show_event(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.open_event(id, Origin::Key, window, cx);
    }

    /// A new event at `start` (the Add button's default when `None`) on the first calendar that is on.
    /// `true` if the form opened.
    pub(super) fn open_new(&mut self, anchor_key: Option<&str>, from: Origin, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // With every calendar switched off a new event could not be seen: nothing opens, and the notice says why.
        let Some(calendar) = self.model.new_event_calendar() else {
            self.model.warn("Every calendar is hidden. Turn one on to add an event.".into());
            cx.notify();
            return false;
        };
        let start = self.model.new_event_start();
        let draft = Draft::new_at(start, calendar, self.model.store.settings.time_format.hour24());
        let anchor = anchor_key.and_then(|key| self.anchor_for(key, from));
        self.open_form(draft, anchor, from, window, cx);
        true
    }

    /// The banner's "Open event".
    pub(super) fn open_next_up(&mut self, from: Origin, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.model.next_up().map(|n| n.id) else { return };
        let Some(e) = self.model.event(id) else { return };
        let draft = Draft::for_event(e, self.model.store.settings.time_format.hour24());
        let anchor = self.anchor_for("banner", from);
        self.open_form(draft, anchor, from, window, cx);
    }

    fn open_form(&mut self, mut draft: Draft, anchor: Option<Bounds<Pixels>>, from: Origin, window: &mut Window, cx: &mut Context<Self>) {
        self.toast = None; // an undo offered earlier no longer applies once something else is edited
        let return_to = self.pop.take().and_then(|f| f.return_to).or_else(|| window.focused(cx)).or_else(|| Some(self.grid_focus()));
        let input = |placeholder: &'static str, value: &str, window: &mut Window, cx: &mut Context<Self>| {
            let value = value.to_string();
            cx.new(|cx| {
                let mut s = InputState::new(window, cx).placeholder(placeholder);
                s.set_value(value, window, cx);
                s
            })
        };
        let textarea = |placeholder: &'static str, value: String, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| {
                let mut s = TextareaState::new(window, cx).placeholder(placeholder);
                s.set_value(value, window, cx);
                s
            })
        };
        let inputs = Inputs {
            notes: textarea("Add notes", std::mem::take(&mut draft.notes), window, cx),
            title: input("Add a title", &draft.title, window, cx),
        };
        let dates = Dates::new(&draft.start_date, &draft.end_date, window, cx);
        let times = Times::new(&draft.start_time, &draft.end_time, self.model.store.settings.time_format.hour24(), window, cx);
        let mut subs = dates.watch(window, cx);
        subs.extend(times.watch(window, cx));
        subs.push(cx.subscribe_in(&inputs.title, window, move |this, _, ev: &InputEvent, window, cx| match ev {
            InputEvent::PressEnter { .. } => this.submit_form(window, cx),
            InputEvent::Change => {
                // An error described the text as it was: it goes once the text changes.
                if let Some(f) = this.pop.as_mut() {
                    f.errors.retain(|e| e.field != Field::Title);
                }
                cx.notify();
            }
            _ => {}
        }));
        self.pop_seq += 1;
        let trap = cx.focus_handle();
        inputs.title.update(cx, |s, cx| s.focus(window, cx));
        self.pop = Some(Form {
            draft,
            errors: Vec::new(),
            inputs,
            dates,
            times,
            anchor,
            by_key: matches!(from, Origin::Key),
            seq: self.pop_seq,
            trap,
            order: RefCell::new(Vec::new()),
            all_day: RefCell::new(None),
            return_to,
            _subs: subs,
        });
        cx.notify();
    }

    /// Close without saving; focus goes back to where it was.
    pub fn close_popover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(f) = self.pop.take() else { return };
        f.return_to.unwrap_or_else(|| self.grid_focus()).focus(window, cx);
        cx.notify();
    }

    pub fn submit_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.pop.as_mut() else { return };
        let draft = form.read(cx);
        match draft.check(self.model.next_event_id(), self.model.store.settings.time_format.hour24()) {
            Err(errors) => {
                let first = errors[0].field;
                form.errors = errors;
                match form.inputs.of(first) {
                    Some(input) => input.update(cx, |s, cx| s.focus(window, cx)),
                    None => form.dates.focus(first, cx).or_else(|| form.times.focus(first, cx)).iter().for_each(|h| h.focus(window, cx)),
                }
            }
            Ok(event) => {
                let id = event.id;
                let edit = self.model.commit_event(event);
                self.undo.push(edit);
                self.month.roving = Some(id);
                self.week.roving = Some(id);
                self.close_popover(window, cx);
                self.sync(cx);
            }
        }
        cx.notify();
    }

    pub fn delete_from_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.pop.as_ref().and_then(|f| f.draft.id) else { return };
        let Some(edit) = self.model.delete_event(id) else { return };
        self.undo.push(edit);
        for roving in [&mut self.month.roving, &mut self.week.roving] {
            if *roving == Some(id) {
                *roving = None;
            }
        }
        self.close_popover(window, cx);
        self.show_toast("Event deleted.", cx);
        self.sync(cx);
    }

    /// Drags are not undoable, so one makes the history and the toast stale: they would replay over it.
    pub(super) fn forget_undo(&mut self) {
        self.toast = None;
        self.undo.clear();
    }

    /// Undo the last delete or save.
    pub fn undo_last(&mut self, cx: &mut Context<Self>) {
        self.toast = None;
        for edit in self.undo.undo().unwrap_or_default() {
            self.model.replay(&edit, true);
        }
        self.sync(cx);
    }

    fn show_toast(&mut self, text: &str, cx: &mut Context<Self>) {
        self.toast = Some(text.to_string());
        self.toast_gen += 1;
        let stamp = self.toast_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(TOAST_SECS)).await;
            this.update(cx, |s, cx| {
                if s.toast_gen == stamp && s.toast.take().is_some() {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn set_form_calendar(&mut self, id: u8, cx: &mut Context<Self>) {
        if let Some(f) = self.pop.as_mut() {
            f.draft.calendar = id;
            cx.notify();
        }
    }

    pub(super) fn toggle_form_all_day(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let hour24 = self.model.store.settings.time_format.hour24();
        let Some(f) = self.pop.as_mut() else { return };
        let mut d = f.read(cx);
        d.set_all_day(!d.all_day, hour24);
        for (field, text) in [(Field::StartTime, d.start_time.clone()), (Field::EndTime, d.end_time.clone())] {
            f.times.set(field, &text, window, cx);
        }
        f.draft.all_day = d.all_day;
        f.errors.clear();
        let in_time = [Field::StartTime, Field::EndTime].into_iter().any(|t| f.times.focus(t, cx).is_some_and(|h| h.contains_focused(window, cx)));
        if d.all_day && in_time {
            f.all_day.borrow().iter().for_each(|h| h.focus(window, cx));
        }
        cx.notify();
    }

    /// Tab and Shift-Tab stay inside the card: the stops (recorded by the last render) in reading order, wrapped.
    pub(super) fn trap_tab(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(f) = &self.pop else { return };
        let order = f.order.borrow();
        let n = order.len();
        let to = match order.iter().position(|h| h.contains_focused(window, cx)) {
            Some(i) if back => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
            None if back => n.saturating_sub(1),
            None => 0,
        };
        if let Some(h) = order.get(to) {
            h.focus(window, cx);
        }
    }
}
