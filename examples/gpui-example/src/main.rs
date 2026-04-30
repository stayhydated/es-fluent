#![allow(clippy::derive_partial_eq_without_eq)]

use example_shared_lib::{ButtonState, CurrentLanguage, Languages};
use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, KeyBinding, Window, WindowBounds, WindowOptions,
    actions,
};
use gpui_component::{button::Button, label::Label};
use gpui_example::{GpuiScreenMessages, i18n};

mod i18n_global {
    use super::i18n;

    pub struct CurrentI18n(pub i18n::I18n);

    impl gpui::Global for CurrentI18n {}
}

actions!(gpui_example, [CycleLocale]);

fn main() {
    let app = gpui_platform::application();
    app.run(|cx: &mut App| {
        let default_language = Languages::default();
        let i18n = i18n::try_new_with_language(default_language).expect("i18n should initialize");
        cx.set_global(CurrentLanguage(default_language));
        cx.set_global(i18n_global::CurrentI18n(i18n));
        cx.bind_keys([KeyBinding::new("t", CycleLocale, Some("GpuiExample"))]);
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, gpui::size(gpui::px(640.), gpui::px(480.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(GpuiExampleView::new);
                view.focus_handle(cx).focus(window, cx);
                view
            },
        )
        .unwrap();
    });
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

        gpui::div()
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
                        gpui::MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.button_state = ButtonState::Pressed;
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _event, _window, cx| {
                            this.button_state = ButtonState::Hovered;
                            cx.notify();
                        }),
                    ),
            )
            .child(
                gpui::div().child(
                    Label::new(
                        i18n.localize_message(&GpuiScreenMessages::ToggleLanguageHint {
                            current_language,
                        }),
                    )
                    .text_color(gpui::white()),
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
