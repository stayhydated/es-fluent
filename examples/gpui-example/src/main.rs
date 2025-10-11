use es_fluent::ToFluentString;
use gpui::{
    App, Application, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px,
    size,
};
use gpui_component::button::Button;
use shared_lib::{ButtonState, CurrentLanguage, Languages, ScreenMessages};
use strum::IntoEnumIterator;
mod i18n;

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.set_global(CurrentLanguage(Languages::default()));
        gpui_component::init(cx);
        i18n::init();
        i18n::change_locale(&Languages::English.into()).unwrap();

        let bounds = Bounds::centered(None, size(px(640.), px(480.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| GpuiExampleView::new(cx)),
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
            .on_key_down(move |event, _window, cx| {
                if event.keystroke.key == "t" {
                    let mut languages: Vec<Languages> = Languages::iter().collect();
                    languages.sort_by_key(|a| *a as isize);
                    let current_index = languages
                        .iter()
                        .position(|&lang| lang == current_language)
                        .unwrap_or(0);
                    let next_index = (current_index + 1) % languages.len();
                    let next_language = languages[next_index];

                    cx.set_global(CurrentLanguage(next_language));
                    i18n::change_locale(&next_language.into()).unwrap();
                }
            })
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
                            this.button_state = ButtonState::Normal;
                            cx.notify();
                        }),
                    ),
            )
            .child(ScreenMessages::ToggleLanguageHint { current_language }.to_fluent_string())
    }
}
