//! The root view: window options, theme sync, and the page layout.

use std::time::Duration;

use chrono::NaiveDateTime;
use gpui_kit::component::Theme as KitTheme;
use gpui_kit::{
    ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Render, Role, StatefulInteractiveElement, Styled, TestSupportExt,
    Window, WindowAppearance, WindowOptions, div, px, size,
};

use super::controls::{c, control, elevation};
use super::model::{Model, NO_PATH, Screen, load_note_text};
use crate::store::Store;
use crate::theme::gpui_adapter::color_tokens;
use crate::theme::resolve::Theme;

/// Guide 5: minimum window 960 by 600; the sidebar becomes a rail below `RAIL_BELOW`.
pub const MIN_WINDOW: (f32, f32) = (960., 600.);
pub const RAIL_BELOW: f32 = 1120.;

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
    clock: Clock,
    /// The theme last handed to gpui-kit, so unchanged frames do not push it again.
    pushed: Option<Theme>,
    pushes: usize,
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
        let dark = is_dark(window.appearance());
        let now = clock();
        let shell = Self { model: Model::new(store, path, now.date(), now.time(), dark), clock, pushed: None, pushes: 0 };
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

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t: Theme = self.model.theme();
        let narrow = window.viewport_size().width < px(RAIL_BELOW);
        let notice = self.notice_bar(&t, window, cx);
        let main = match self.model.screen {
            Screen::Appearance => self.appearance_screen(&t, window, cx).into_any_element(),
            Screen::Calendar => div()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(self.header(&t, window, cx))
                .child(div().id("main-content").flex_1())
                .into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .gap(px(16.))
            .p(px(16.))
            .bg(c(t.neutral.page))
            .text_color(c(t.neutral.t1))
            .text_size(px(13.))
            .font_weight(FontWeight::NORMAL)
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
                    .children(notice)
                    .child(main),
            )
    }
}

