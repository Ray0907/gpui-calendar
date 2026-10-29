//! The start and end times of the popover: picked from a list of times at 15-minute steps, never typed. The field is a box like the day
//! field's (`date.rs`); a click anywhere on it, or Up, Down, Enter, opens the list, and the focus stays on the field while Up, Down, Enter
//! and Escape drive the list.

use chrono::NaiveTime;
use std::cell::Cell;

use gpui_kit::{
    Anchor, AnyElement, App, ClickEvent, Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, Modifiers, MouseButton, ParentElement, Role, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, TestSupportExt, Window, anchored, deferred, div, prelude::*, px,
};

#[cfg(any(test, feature = "test-hooks"))]
use gpui_kit::ElementId;

use super::Form;
use super::fields::{FIELD_H, FIELD_R, error_note, label};
use crate::domain::dates::format_time;
use crate::domain::form::{Field, parse_date, parse_time};
use crate::domain::timelist::{duration, end_rows, start_rows};
use crate::theme::resolve::Theme;
use crate::ui::a11y::Disabled;
use crate::ui::controls::{c, field_hover, ghost_hover, ring, row_press, Pointer};
use crate::ui::shell::Shell;

const ROW_H: f32 = 32.;
const LIST_W: f32 = 176.;
/// Seven rows tall (a half row more, so the list is seen to scroll).
const LIST_H: f32 = 7.5 * ROW_H;

struct Slot {
    time: NaiveTime,
    /// The draft's own text for a time that cannot be read (the field sits at midnight). It is shown, kept and refused on save until a time is picked.
    unread: Option<String>,
    focus: FocusHandle,
    open: bool,
    /// The row the keyboard is on while the list is open.
    cursor: usize,
    scroll: ScrollHandle,
    /// Frames the open list has been drawn: the scroll to the value needs the list's own bounds, which exist from the second frame on.
    settle: Cell<u8>,
}

/// The two fields; each starts on the draft's time, in the app's 12 or 24 hour clock.
pub(super) struct Times {
    slots: [Slot; 2],
    hour24: bool,
    /// A press landed on a time field this event: the card's own press handler, which runs after it, leaves the lists alone.
    on_field: bool,
}

impl Times {
    pub fn new(start: &str, end: &str, hour24: bool, cx: &mut Context<Shell>) -> Self {
        let make = |text: &str, cx: &mut Context<Shell>| {
            let time = parse_time(text, hour24);
            Slot { time: time.unwrap_or(NaiveTime::MIN), unread: time.is_none().then(|| text.to_string()), focus: cx.focus_handle(), open: false, cursor: 0, scroll: ScrollHandle::new(), settle: Cell::new(0) }
        };
        Self { slots: [make(start, cx), make(end, cx)], hour24, on_field: false }
    }

    fn slot(&self, f: Field) -> &Slot {
        &self.slots[usize::from(f == Field::EndTime)]
    }

    fn slot_mut(&mut self, f: Field) -> &mut Slot {
        &mut self.slots[usize::from(f == Field::EndTime)]
    }

    pub fn has(&self, f: Field) -> bool {
        matches!(f, Field::StartTime | Field::EndTime)
    }

    /// True while the draft's own text for `f` stands because it could not be read.
    pub fn is_unread(&self, f: Field) -> bool {
        self.slot(f).unread.is_some()
    }

    /// The time shown, `14:30` or `2:30 PM`; the draft's own text while no time is picked.
    pub fn text(&self, f: Field) -> String {
        let s = self.slot(f);
        s.unread.clone().unwrap_or_else(|| format_time(s.time, self.hour24))
    }

    /// Show `text` as picking it would: a time that reads is set, one that does not is kept as the draft's text.
    pub fn set(&mut self, f: Field, text: &str) {
        let (time, s) = (parse_time(text, self.hour24), self.slot_mut(f));
        s.time = time.unwrap_or(s.time);
        s.unread = time.is_none().then(|| text.to_string());
        s.open = false;
    }

    pub fn focus(&self, f: Field) -> Option<FocusHandle> {
        self.has(f).then(|| self.slot(f).focus.clone())
    }

    /// A list closes when its field loses the focus (Tab, or a click elsewhere in the card).
    pub fn watch(&self, window: &mut Window, cx: &mut Context<Shell>) -> Vec<Subscription> {
        [Field::StartTime, Field::EndTime]
            .into_iter()
            .map(|field| {
                cx.on_blur(&self.slot(field).focus, window, move |this, _, cx| {
                    if let Some(f) = this.pop.as_mut() {
                        f.times.slot_mut(field).open = false;
                    }
                    cx.notify();
                })
            })
            .collect()
    }
}

impl Form {
    /// The rows of `f`'s list and, for the end list on one day, the minutes after the start.
    fn time_rows(&self, f: Field, cx: &App) -> Vec<(NaiveTime, Option<i64>)> {
        let current = self.times.slot(f).time;
        if f == Field::StartTime {
            return start_rows(current).into_iter().map(|t| (t, None)).collect();
        }
        let start = self.times.slot(Field::StartTime).time;
        // Parsed, not compared as text: two unreadable drafts are not one day.
        let same_day = matches!((parse_date(&self.dates.text(Field::StartDate, cx)), parse_date(&self.dates.text(Field::EndDate, cx))), (Some(a), Some(b)) if a == b);
        end_rows(start, current, same_day).into_iter().map(|t| (t, same_day.then(|| (t - start).num_minutes()))).collect()
    }
}

impl Shell {
    /// A press on the card outside every time field and list closes the lists; the focus may not have left the field.
    pub(in crate::ui) fn card_pressed(&mut self, cx: &mut Context<Self>) {
        if self.pop.as_mut().is_some_and(|f| std::mem::take(&mut f.times.on_field)) {
            return;
        }
        self.close_time_list(cx);
    }

    /// Escape closes an open list first. `true` if one was open.
    pub(in crate::ui) fn close_time_list(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(f) = self.pop.as_mut() else { return false };
        let was = f.times.slots.iter().any(|s| s.open);
        f.times.slots.iter_mut().for_each(|s| s.open = false);
        if was {
            cx.notify();
        }
        was
    }

    fn open_time_list(&mut self, field: Field, cx: &mut Context<Self>) {
        let Some(f) = self.pop.as_mut() else { return };
        f.times.slots.iter_mut().for_each(|s| s.open = false);
        let rows = f.time_rows(field, cx);
        // A time nobody can read has no row: open at the working day's start, not at midnight.
        let at = if f.times.is_unread(field) { rows.iter().position(|&(t, _)| t >= NaiveTime::from_hms_opt(9, 0, 0).unwrap_or_default()) } else { rows.iter().position(|&(t, _)| t == f.times.slot(field).time) };
        let at = at.unwrap_or(0);
        let s = f.times.slot_mut(field);
        (s.open, s.cursor) = (true, at);
        s.settle.set(0);
        cx.notify();
    }

    /// Pick `time`: the draft takes it, the list closes, and the error that described the old time goes.
    fn pick_time(&mut self, field: Field, time: NaiveTime, cx: &mut Context<Self>) {
        let Some(f) = self.pop.as_mut() else { return };
        let s = f.times.slot_mut(field);
        (s.time, s.unread, s.open) = (time, None, false);
        f.errors.retain(|e| e.field != field);
        cx.notify();
    }

    /// Up and Down open the list, then move through it; Enter and Space open it, then pick the row the keyboard is on.
    fn time_key(&mut self, field: Field, key: &str, mods: Modifiers, cx: &mut Context<Self>) -> bool {
        let Some(f) = self.pop.as_mut() else { return false };
        // Cmd+Enter and the like belong to the shortcuts outside the field.
        if mods.platform || mods.control || mods.alt || !matches!(key, "up" | "down" | "enter" | "space") {
            return false;
        }
        if !f.times.slot(field).open {
            self.open_time_list(field, cx);
            return true;
        }
        let rows = f.time_rows(field, cx);
        let s = f.times.slot_mut(field);
        match key {
            "up" | "down" => {
                s.cursor = if key == "up" { s.cursor.saturating_sub(1) } else { (s.cursor + 1).min(rows.len().saturating_sub(1)) };
                s.scroll.scroll_to_item(s.cursor);
                cx.notify();
            }
            _ => {
                if let Some(&(t, _)) = rows.get(s.cursor) {
                    self.pick_time(field, t, cx);
                }
            }
        }
        true
    }

    /// A labelled time field, 48px like the others; disabled (greyed, not in the tab order) while the event is all day. An error
    /// sits under it like a text field's. The box is a combobox: the whole of it takes the click, and its keyboard ring shows only
    /// for keyboard focus, as the day field's does.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn time_field(&self, t: &Theme, f: Field, (name, id): (&'static str, &'static str), form: &Form, calm: bool, window: &Window, cx: &Context<Self>) -> AnyElement {
        let (error, off, n) = (form.error(f), form.draft.all_day, &t.neutral);
        let slot = form.times.slot(f);
        let (open, text) = (slot.open && !off, form.times.text(f));
        let keyboard = window.last_input_was_keyboard() && slot.focus.is_focused(window);
        let trigger = Disabled::new(div().id(SharedString::from(format!("{id}-trigger"))), off)
            .test_support()
            .role(Role::ComboBox)
            .aria_label(format!("{name}, {text}"))
            .aria_expanded(open)
            .track_focus(&slot.focus.clone().tab_stop(!off))
            .h(px(FIELD_H))
            .flex()
            .items_center()
            .px(px(12.))
            .rounded(px(FIELD_R))
            .border_1()
            // A time the draft held but nobody can read sits at 00:00: invalid (the `ink` outline) until one is picked.
            .border_color(if keyboard { c(t.ring) } else { c(n.border) })
            .when(error.is_some() || form.times.is_unread(f), |d| d.border_2().border_color(c(t.ink)))
            .bg(c(n.field))
            .text_color(c(n.t1))
            .when(keyboard, |d| d.shadow(ring(c(n.surface), c(t.ring))))
            .when(off, |d| d.opacity(0.5))
            .when(!off, |d| {
                d.pointer()
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, _| this.pop.as_mut().map_or((), |f| f.times.on_field = true)))
                    .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                        // Enter and Space also arrive here as clicks; `on_key_down` has already handled them.
                        if e.is_keyboard() {
                            return;
                        }
                        let Some(form) = this.pop.as_ref() else { return };
                        let was = form.times.slot(f).open;
                        form.times.slot(f).focus.focus(window, cx);
                        if was { this.close_time_list(cx); } else { this.open_time_list(f, cx); }
                    }))
                    .on_key_down(cx.listener(move |this, e: &KeyDownEvent, _, cx| {
                        if this.time_key(f, &e.keystroke.key, e.keystroke.modifiers, cx) {
                            cx.stop_propagation();
                        }
                    }))
            })
            .child(text);
        let list = open.then(|| self.time_list(t, f, id, form, window, cx));
        Disabled::new(div().id(SharedString::from(format!("{id}-field"))), off)
            .test_support()
            .role(Role::Group)
            .aria_label(name)
            .when_some(error, |d, e| d.aria_description(e))
            .flex_1()
            .min_w_0()
            .mb(px(if error.is_some() { 4. } else { 16. }))
            .child(label(t, name))
            .child(field_hover(t, format!("{id}-box"), px(FIELD_R), calm && !off && error.is_none() && !form.times.is_unread(f) && !open, div().relative().child(trigger).children(list)))
            .when_some(error, |d, e| d.child(error_note(t, id, e)))
            .into_any_element()
    }

    /// The open list: one row per time, the selected one in `ink`, the keyboard's row and the hovered row in `wash-hover` (one fill, never two stacked).
    fn time_list(&self, t: &Theme, f: Field, id: &'static str, form: &Form, window: &Window, cx: &Context<Self>) -> AnyElement {
        let (n, slot) = (&t.neutral, form.times.slot(f));
        // Frame 1 lays the list out (hidden, so it does not flash at the top), frame 2 scrolls the value to the top, frame 3 shows it.
        let frame = slot.settle.get();
        match frame {
            0 => window.request_animation_frame(),
            1 => {
                slot.scroll.scroll_to_top_of_item(slot.cursor);
                window.request_animation_frame();
            }
            _ => {}
        }
        slot.settle.set((frame + 1).min(2));
        let hour24 = form.times.hour24;
        let rows = form.time_rows(f, cx).into_iter().enumerate().map(|(i, (time, after))| {
            let (selected, at_cursor) = (time == slot.time && slot.unread.is_none(), i == slot.cursor);
            let clock = format_time(time, hour24);
            let name = after.map_or_else(|| clock.clone(), |m| format!("{clock}, {}", duration(m)));
            div()
                .id(SharedString::from(format!("{id}-opt-{}", time.format("%H%M"))))
                .test_support()
                .role(Role::ListBoxOption)
                .aria_label(name)
                .aria_selected(selected)
                .h(px(ROW_H))
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .px(px(12.))
                .rounded(px(6.))
                .pointer()
                .text_color(if selected { c(t.on_ink) } else { c(n.t1) })
                .when(selected, |d| d.bg(c(t.ink)))
                .when(!selected && at_cursor, |d| d.bg(c(crate::theme::contrast::wash_hover_rgb(n.t1, n.surface))))
                .when(!selected, |d| d.hover(ghost_hover(t, n.surface)).active(row_press(t, n.surface)))
                .on_click(cx.listener(move |this, _, _, cx| this.pick_time(f, time, cx)))
                .child(div().child(clock))
                .children(after.map(|m| div().text_size(px(12.)).font_weight(FontWeight::NORMAL).text_color(if selected { c(t.on_ink) } else { c(n.t3) }).child(duration(m))))
        });
        let list = div()
            .id(SharedString::from(format!("{id}-list")))
            .test_support()
            .role(Role::ListBox)
            .aria_label(format!("{} times", id.trim_start_matches("pop-").trim_end_matches("-time")))
            .occlude() // a click on the list is not a click on the card behind it
            .mt(px(4.))
            .w(px(LIST_W))
            .h(px(LIST_H))
            .p(px(4.))
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&slot.scroll)
            .rounded(px(FIELD_R))
            .border_1()
            .border_color(c(n.line))
            .bg(c(n.surface))
            .shadow(crate::ui::controls::elevation(t))
            .text_size(px(13.))
            .when(frame < 2, |d| d.invisible())
            .children(rows);
        deferred(anchored().anchor(Anchor::TopLeft).snap_to_window_with_margin(px(8.)).child(list)).with_priority(4).into_any_element()
    }

    /// The element id of a time field's box, for tests.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn popover_time_id(&self, f: Field) -> Option<ElementId> {
        self.pop.as_ref()?.times.has(f).then(|| SharedString::from(if f == Field::StartTime { "pop-start-time-trigger" } else { "pop-end-time-trigger" }).into())
    }

    /// The focus handle of a time field, for tests.
    #[cfg(any(test, feature = "test-hooks"))]
    pub fn popover_time_focus(&self, f: Field, _: &App) -> Option<FocusHandle> {
        self.pop.as_ref()?.times.focus(f)
    }
}
