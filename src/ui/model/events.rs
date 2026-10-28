//! Editing events from the form: each change is applied to the store and saved, and comes back as an
//! `Edit` the undo history can replay.

use super::{Model, Save};
use crate::domain::event::Event;
use crate::domain::form::{Edit, default_start, next_id, remove, save};
use chrono::NaiveDateTime;

impl Model {
    pub fn event(&self, id: u64) -> Option<&Event> {
        self.store.events.iter().find(|e| e.id == id)
    }

    pub fn next_event_id(&self) -> u64 {
        next_id(&self.store.events)
    }

    /// Where a new event starts when nothing was clicked: the shown day, at the next free hour today.
    pub fn new_event_start(&self) -> NaiveDateTime {
        default_start(self.anchor, self.today.and_time(self.now))
    }

    /// Add or replace `event`, then save.
    pub fn commit_event(&mut self, event: Event) -> Edit {
        let edit = save(&mut self.store.events, event);
        self.persist();
        edit
    }

    pub fn delete_event(&mut self, id: u64) -> Option<Edit> {
        let edit = remove(&mut self.store.events, id)?;
        self.persist();
        Some(edit)
    }

    /// Take `edit` back (`undo`) or do it again, then save.
    pub fn replay(&mut self, edit: &Edit, undo: bool) -> Save {
        if undo {
            edit.revert(&mut self.store.events);
        } else {
            edit.apply(&mut self.store.events);
        }
        self.persist()
    }
}
