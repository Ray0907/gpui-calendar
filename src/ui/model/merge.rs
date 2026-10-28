//! Retrying a blocked load: the viewer's changes are laid back on top of the file that was read.

use super::Model;
use crate::store::Store;
use crate::theme::rules::repair;

/// `*v = mine` when the viewer changed the field since `base`.
fn keep<T: PartialEq>(base: T, mine: T, v: &mut T, changed: &mut bool) {
    if base != mine {
        *v = mine;
        *changed = true;
    }
}

impl Model {
    /// Put onto the loaded store every field of `mine` that differs from `base`. `true` if any did.
    pub(super) fn merge_changes(&mut self, base: &Store, mine: &Store) -> bool {
        let (b, m) = (base.settings.appearance, mine.settings.appearance);
        let mut a = self.store.settings.appearance;
        let mut changed = false;
        keep(b.mode, m.mode, &mut a.mode, &mut changed);
        keep(b.accent, m.accent, &mut a.accent, &mut changed);
        keep(b.neutral, m.neutral, &mut a.neutral, &mut changed);
        keep(b.calendar_colors.work, m.calendar_colors.work, &mut a.calendar_colors.work, &mut changed);
        keep(b.calendar_colors.personal, m.calendar_colors.personal, &mut a.calendar_colors.personal, &mut changed);
        keep(b.calendar_colors.side, m.calendar_colors.side, &mut a.calendar_colors.side, &mut changed);
        for c in &mut self.store.calendars {
            let was = |s: &Store| s.calendars.iter().find(|k| k.id == c.id).map(|k| k.visible);
            if let (Some(bv), Some(mv)) = (was(base), was(mine)) {
                keep(bv, mv, &mut c.visible, &mut changed);
            }
        }
        self.store.settings.appearance = repair(a);
        changed
    }
}
