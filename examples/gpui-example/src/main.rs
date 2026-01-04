use es_fluent::{EsFluent, ToFluentString as _};
use example_shared_lib::{ButtonState, CurrentLanguage, Languages};
use gpui::{
    App, Application, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    size,
};
use gpui_component::button::Button;
use gpui_example::i18n;

#[derive(Clone, Copy, Debug, EsFluent)]
pub enum GpuiScreenMessages {
    ToggleLanguageHint { current_language: Languages },
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let default_language = Languages::default();
        cx.set_global(CurrentLanguage(default_language));
        gpui_component::init(cx);
        i18n::init();
        i18n::change_locale(default_language).unwrap();

        let bounds = Bounds::centered(None, size(px(640.), px(480.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(GpuiExampleView::new),
        )
        .unwrap();
    });
}

struct GpuiExampleView {
    button_state: ButtonState,
}

impl GpuiExampleView {
    fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            button_state: ButtonState::Normal,
        }
    }
}

impl Render for GpuiExampleView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_language = cx.global::<CurrentLanguage>().0;

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
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
                        Button::new("toggle-language-button")
                            .label(
                                GpuiScreenMessages::ToggleLanguageHint { current_language }
                                    .to_fluent_string(),
                            )
                            .on_click(cx.listener(|_this, _event, _window, cx| {
                                let current_language = cx.global::<CurrentLanguage>().0;
                                let new_lang = current_language.next();
                                cx.set_global(CurrentLanguage(new_lang));
                                i18n::change_locale(new_lang).unwrap();
                            })),
                    ),
            )
    }
}
