//! The root view: window options, theme sync, and the page layout.

use std::time::Duration;

use chrono::NaiveDateTime;
use gpui_kit::component::Theme as KitTheme;
use gpui_kit::{
    ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, Role, StatefulInteractiveElement, Styled, TestSupportExt,
    Window, WindowAppearance, WindowOptions, div, px, size,
};

use super::controls::{c, control, elevation};
use super::popover::{Origin, anchor_mark, anchor_reset};
use super::model::{Command, Model, NO_PATH, Screen, View, command, load_note_text};
use crate::store::Store;
use crate::theme::gpui_adapter::color_tokens;
use crate::theme::resolve::Theme;

/// Guide 5: minimum window 960 by 600; the sidebar becomes a rail below `RAIL_BELOW`.
pub const MIN_WINDOW: (f32, f32) = (960., 600.);
pub const RAIL_BELOW: f32 = 1120.;
/// Padding around the window's content and the gap between sidebar and card.
const MARGIN: f32 = 16.;
/// What the shell spends above and below a view's body: its margins and the header.
pub const CHROME_H: f32 = 2. * MARGIN + super::header::HEADER_H;

pub fn window_options() -> WindowOptions {
    WindowOptions {
        window_min_size: Some(size(px(MIN_WINDOW.0), px(MIN_WINDOW.1))),
        ..Default::default()
    }
}

/// Where "now" comes from: local time in the app, a settable cell in tests.
pub type Clock = Box<dyn Fn() -> NaiveDateTime>;

pub fn local_clock() -> Clock {
    Box::new(|| chrono::Local::now().naive_local())
}

/// How often the clock is read; also read whenever the window is activated.
const TICK: Duration = Duration::from_secs(30);

/// The one place that turns the OS appearance into the system dark flag.
pub(super) fn is_dark(a: WindowAppearance) -> bool {
    !matches!(a, WindowAppearance::Light | WindowAppearance::VibrantLight)
}

pub struct Shell {
    pub model: Model,
    /// Focus when nothing inside has it, so the shortcuts have a place to arrive.
    root: gpui_kit::FocusHandle,
    clock: Clock,
    /// The theme last handed to gpui-kit, so unchanged frames do not push it again.
    pushed: Option<Theme>,
    pushes: usize,
    pub(super) month: super::month::MonthView,
    pub(super) week: super::week::WeekView,
    pub(super) year: super::year::YearView,
    pub(super) pop: Option<super::popover::Form>,
    pub(super) pop_seq: usize,
    pub(super) anchors: super::popover::Anchors,
    /// Deletes and saves, for the toast's Undo.
    pub(super) undo: gpui_kit::base::UndoHistory<crate::domain::form::Edit>,
    pub(super) toast: Option<String>,
    pub(super) toast_gen: u64,
}

impl Shell {
    /// Load the store from `path` as the app does and build the shell; what the load found (a file set
    /// aside or repaired, a read error, nowhere to save) is said in the notice bar.
    pub fn start(path: Option<std::path::PathBuf>, clock: Clock, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let loaded = path.as_deref().map(Store::load_noted);
        let (store, note, error) = match loaded {
            Some(Ok((store, note))) => (store, note, None),
            Some(Err(e)) => (Store::default(), None, Some(e.to_string())),
            None => (Store::default(), None, None),
        };
        let mut shell = Self::new(store, path.clone(), clock, window, cx);
        if path.is_none() {
            shell.model.warn(NO_PATH.into());
        }
        if let Some(note) = note {
            shell.model.warn(load_note_text(note));
        }
        if let Some(why) = error {
            shell.block_saving(why);
        }
        shell
    }

    pub fn new(store: Store, path: Option<std::path::PathBuf>, clock: Clock, window: &mut Window, cx: &mut Context<Self>) -> Self {
        super::popover::bind_keys(cx);
        let dark = is_dark(window.appearance());
        let now = clock();
        let model = Model::new(store, path, now.date(), now.time(), dark);
        let month = super::month::MonthView::new(now.date(), model.week_start(), cx);
        let week = super::week::WeekView::new(cx);
        let year = super::year::YearView::new(cx);
        let root = cx.focus_handle();
        window.focus(&root, cx);
        let shell = Self {
            model,
            root,
            clock,
            pushed: None,
            pushes: 0,
            month,
            week,
            year,
            pop: None,
            pop_seq: 0,
            anchors: Default::default(),
            undo: gpui_kit::base::UndoHistory::new(),
            toast: None,
            toast_gen: 0,
        };
        cx.observe_window_appearance(window, |this, window, cx| {
            this.model.system_dark = is_dark(window.appearance());
            this.sync(cx);
        })
        .detach();
        cx.observe_window_activation(window, |this, _, cx| this.tick(cx)).detach();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let mut shell = shell;
        shell.push_theme(cx);
        shell
    }

    /// Read the clock; "today" and the next-up banner follow it.
    fn tick(&mut self, cx: &mut Context<Self>) {
        self.model.set_now((self.clock)());
        cx.notify();
    }

    /// The settings file could not be read: keep it, save nothing, say so.
    pub fn block_saving(&mut self, why: String) {
        self.model.block_saving(why);
    }

    /// How many times the theme went to gpui-kit.
    pub fn theme_pushes(&self) -> usize {
        self.pushes
    }

    /// Run after any model change: re-render and hand the new colors to gpui-kit's global theme.
    pub fn sync(&mut self, cx: &mut Context<Self>) {
        self.push_theme(cx);
        cx.notify();
    }

    fn push_theme(&mut self, cx: &mut Context<Self>) {
        let theme = self.model.theme();
        if self.pushed == Some(theme) {
            return;
        }
        self.pushed = Some(theme);
        self.pushes += 1;
        KitTheme::update(cx, |k| {
            let mut tokens = k.semantic_tokens();
            tokens.colors = color_tokens(&theme);
            k.apply_semantic_tokens(&tokens);
        });
    }
}

impl Shell {
    /// Save problems and a blocked store, in the flow above the content so nothing is covered.
    fn notice_bar(&mut self, t: &Theme, window: &mut Window, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        if self.model.notice.is_empty() {
            return None;
        }
        let msg = self.model.notice.clone();
        let button = |id: &'static str, label: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            control(id, Role::Button, label, c(t.ink), c(t.on_ink), window, cx)
                .h(px(28.))
                .px(px(10.))
                .flex()
                .items_center()
                .flex_none()
                .rounded(px(8.))
                .border_1()
                .border_color(c(t.on_ink))
                .font_weight(FontWeight::SEMIBOLD)
                .child(label)
        };
        let retry = self.model.can_retry().then(|| {
            button("notice-retry", "Retry loading settings", window, cx).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.model.retry_load();
                this.sync(cx);
            }))
        });
        let dismiss = button("notice-dismiss", "Dismiss notice", window, cx).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.model.dismiss_notice();
            this.sync(cx);
        }));
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(16.))
                .py(px(8.))
                .bg(c(t.ink))
                .text_color(c(t.on_ink))
                .text_size(px(12.))
                .child(div().id("notice").test_support().role(Role::Status).aria_label(msg.clone()).flex_1().min_w_0().child(msg))
                .children(retry)
                .child(dismiss),
        )
    }
}

impl Shell {
    /// Run the global shortcut `key` stands for. Instant: no animation starts from the keyboard.
    /// `true` if it was one.
    fn shortcut(&mut self, key: &str, modified: bool, held: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        // A field or the popover owns the keyboard, and the Appearance screen has no calendar behind it.
        let blocked = self.pop.is_some() || self.model.screen != Screen::Calendar;
        let Some(cmd) = command(key, modified, held, blocked) else { return false };
        match cmd {
            // A refusal is said in the notice bar and leaves the key to whoever else wants it.
            Command::NewEvent => return self.open_new(None, Origin::Key, window, cx),
            cmd => self.run(cmd, cx),
        }
        true
    }

    /// Navigate: the one path the keys and the header buttons share, so they cannot drift apart.
    pub(super) fn run(&mut self, cmd: Command, cx: &mut Context<Self>) {
        match cmd {
            Command::Today => self.model.go_today(),
            Command::Show(v) => self.model.set_view(v),
            Command::Step(dir) => self.model.step(dir),
            Command::NewEvent => return,
        }
        self.sync(cx);
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t: Theme = self.model.theme();
        let narrow = window.viewport_size().width < px(RAIL_BELOW);
        let notice = self.notice_bar(&t, window, cx);
        let main = match self.model.screen {
            Screen::Appearance => self.appearance_screen(&t, window, cx).into_any_element(),
            Screen::Calendar => {
                if !matches!(self.model.view, View::Day | View::Week) {
                    self.week.closed();
                }
                let body = match self.model.view {
                    View::Month => {
                        self.month_sync();
                        self.month_view(&t, narrow, window, cx)
                    }
                    View::Day | View::Week => self.week_view(&t, window, cx),
                    View::Year => self.year_view(&t, narrow, window, cx),
                };
                div()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(self.header(&t, window, cx))
                    .child(body)
                    .into_any_element()
            }
        };
        // Whatever had focus may be gone once this frame is drawn (a view, screen, popover, toast or notice
        // button left the tree): the shortcuts need a place to arrive, so focus falls back to the root.
        let root = self.root.clone();
        window.defer(cx, move |window, cx| {
            if !root.contains_focused(window, cx) {
                root.focus(window, cx);
            }
        });
        let popover = self.popover_layer(&t, window, cx);
        let toast = self.toast_layer(&t, window, cx);
        div()
            .size_full()
            .track_focus(&self.root)
            .flex()
            .gap(px(MARGIN))
            .p(px(MARGIN))
            .bg(c(t.neutral.page))
            .text_color(c(t.neutral.t1))
            .text_size(px(13.))
            .font_weight(FontWeight::NORMAL)
            // Esc closes the popover wherever focus is, before any grid or field sees it. gpui runs key bindings
            // before key-down listeners: an open date picker takes its Cancel and stops it, so only a closed one
            // (which hands it on) lets Esc get here. One Esc therefore never closes both.
            .capture_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                if e.keystroke.key == "escape" && this.pop.is_some() {
                    this.close_popover(window, cx);
                    cx.stop_propagation();
                }
            }))
            // Shortcuts: a grid that used the key has already stopped it.
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, window, cx| {
                if this.shortcut(&e.keystroke.key, e.keystroke.modifiers.modified(), e.is_held, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .child(anchor_reset(&self.anchors))
            .child(self.sidebar(&t, narrow, window, cx))
            .child(
                div()
                    .id("main")
                    .test_support()
                    .flex_1()
                    .min_w_0()
                    .rounded(px(14.))
                    .bg(c(t.neutral.surface))
                    .shadow(elevation(&t))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(anchor_mark(&self.anchors, "main".into()))
                    .children(notice)
                    .child(main),
            )
            .children(popover)
            .children(toast)
    }
}

