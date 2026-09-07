#![allow(clippy::derive_partial_eq_without_eq)]

use std::borrow::Cow;
#[cfg(target_family = "wasm")]
use std::cell::RefCell;

use example_shared_lib::{ButtonState, CurrentLanguage, Languages};
use gpui_example::{GpuiScreenMessages, i18n};
use gpui_kit::component::{ActiveTheme as _, Root, button::Button, label::Label};
use gpui_kit::prelude::*;
use gpui_kit::{App, Context, FocusHandle, Focusable, KeyBinding, Window, WindowOptions, actions};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::{Bounds, WindowBounds};
#[cfg(not(target_family = "wasm"))]
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

mod i18n_global {
    use super::i18n;

    pub struct CurrentI18n(pub i18n::I18n);

    impl gpui_kit::Global for CurrentI18n {}
}

actions!(gpui_example, [CycleLocale]);

#[cfg(not(target_family = "wasm"))]
fn main() {
    init_tracing();
    gpui_kit::application().run(launch);
}

#[cfg(target_family = "wasm")]
thread_local! {
    static APPLICATION: RefCell<Option<gpui_kit::ApplicationHandle>> = const { RefCell::new(None) };
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    gpui_kit::platform::web_init();
    let app = gpui_kit::platform::single_threaded_web();
    APPLICATION.with(|application| {
        *application.borrow_mut() = Some(app.run_embedded(launch));
    });
    Ok(())
}

#[cfg(target_family = "wasm")]
fn main() {}

fn launch(cx: &mut App) {
    gpui_kit::init(cx);
    cx.text_system()
        .add_fonts(vec![Cow::Borrowed(
            include_bytes!("../assets/fonts/NotoSansSC-Bold.ttf").as_slice(),
        )])
        .expect("Failed to load NotoSansSC-Bold font");

    let startup_language = Languages::default();
    let i18n = i18n::try_new_with_language(startup_language).expect("i18n should initialize");
    cx.set_global(CurrentLanguage(startup_language));
    cx.set_global(i18n_global::CurrentI18n(i18n));
    cx.bind_keys([KeyBinding::new("t", CycleLocale, Some("GpuiExample"))]);

    #[cfg(not(target_family = "wasm"))]
    let options = {
        let bounds = Bounds::centered(
            None,
            gpui_kit::size(gpui_kit::px(640.), gpui_kit::px(480.)),
            cx,
        );
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        }
    };
    #[cfg(target_family = "wasm")]
    let options = WindowOptions::default();

    cx.open_window(options, |window, cx| {
        let view = cx.new(GpuiExampleView::new);
        view.focus_handle(cx).focus(window, cx);
        cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
    })
    .expect("the GPUI example window should open");

    cx.activate(true);
}

#[cfg(not(target_family = "wasm"))]
fn init_tracing() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

struct GpuiExampleView {
    button_state: ButtonState,
    focus_handle: FocusHandle,
}

impl GpuiExampleView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            button_state: ButtonState::Normal,
            focus_handle: cx.focus_handle(),
        }
    }

    fn cycle_locale(&mut self, cx: &mut Context<Self>) {
        let current_language = cx.global::<CurrentLanguage>().0;
        let new_lang = current_language.next();
        cx.set_global(CurrentLanguage(new_lang));
        cx.global::<i18n_global::CurrentI18n>()
            .0
            .select_language(new_lang)
            .unwrap();
        cx.notify();
    }
}

impl Focusable for GpuiExampleView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GpuiExampleView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_language = cx.global::<CurrentLanguage>().0;
        let i18n = cx.global::<i18n_global::CurrentI18n>().0.clone();

        gpui_kit::div()
            .font_family("Noto Sans SC")
            .id("gpui-example")
            .key_context("GpuiExample")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &CycleLocale, _window, cx| {
                this.cycle_locale(cx);
            }))
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .items_center()
            .justify_center()
            .child(
                Button::new("state-button")
                    .label(i18n.localize_message(&self.button_state))
                    .on_hover(cx.listener(|this, hovered, _window, cx| {
                        if *hovered {
                            this.button_state = ButtonState::Hovered;
                        } else {
                            this.button_state = ButtonState::Normal;
                        }
                        cx.notify();
                    }))
                    .on_mouse_down(
                        gpui_kit::MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.button_state = ButtonState::Pressed;
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        gpui_kit::MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.button_state = ButtonState::Hovered;
                            cx.notify();
                        }),
                    ),
            )
            .child(
                gpui_kit::div().child(
                    Label::new(
                        i18n.localize_message(&GpuiScreenMessages::ToggleLanguageHint {
                            current_language,
                        }),
                    )
                    .text_color(cx.theme().foreground),
                ),
            )
            .child(
                Button::new("change-locale-button")
                    .label(i18n.localize_message(&GpuiScreenMessages::ChangeLocaleButton))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.cycle_locale(cx);
                    })),
            )
    }
}
