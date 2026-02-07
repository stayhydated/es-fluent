use es_fluent::ToFluentString as _;
use example_shared_lib::{ButtonState, CurrentLanguage, Languages};
use gpui::{
    App, Application, Bounds, Context, FocusHandle, Focusable, KeyBinding, Window, WindowBounds,
    WindowOptions, actions, div, prelude::*, px, size,
};
use gpui_component::{button::Button, label::Label};
use gpui_example::{GpuiScreenMessages, i18n};

actions!(gpui_example, [CycleLocale]);

fn main() {
    Application::new().run(|cx: &mut App| {
        let default_language = Languages::default();
        cx.set_global(CurrentLanguage(default_language));
        cx.bind_keys([KeyBinding::new("t", CycleLocale, Some("GpuiExample"))]);
        gpui_component::init(cx);
        i18n::init();
        i18n::change_locale(default_language).unwrap();

        let bounds = Bounds::centered(None, size(px(640.), px(480.)), cx);
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
        i18n::change_locale(new_lang).unwrap();
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

        div()
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
                    .label(self.button_state.to_fluent_string())
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
                div().child(
                    Label::new(
                        GpuiScreenMessages::ToggleLanguageHint { current_language }
                            .to_fluent_string(),
                    )
                    .text_color(gpui::white()),
                ),
            )
            .child(
                Button::new("change-locale-button")
                    .label(GpuiScreenMessages::ChangeLocaleButton.to_fluent_string())
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.cycle_locale(cx);
                    })),
            )
    }
}
