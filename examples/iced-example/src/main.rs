use es_fluent::{EsFluent, ToFluentString};
use example_shared_lib::{ButtonState, Languages};
use iced::font::{Family, Font};
use iced::widget::{button, center, mouse_area, row, text};
use iced::{Center, Element, Theme};
use strum::IntoEnumIterator;

mod i18n;

const NOTO_SANS_SC: &[u8] = include_bytes!("../../assets/fonts/NotoSansSC-Bold.ttf");
const FONT: Font = Font {
    family: Family::Name("Noto Sans SC"),
    weight: iced::font::Weight::Bold, // normal doesnt render cn? ig this demo gon have ☒
    ..Font::DEFAULT
};

#[derive(Clone, Copy, Debug, EsFluent)]
pub enum IcedScreenMessages {
    ToggleLanguageHint { current_language: Languages },
}

pub fn main() -> iced::Result {
    i18n::init();
    i18n::change_locale(&Languages::English.into()).unwrap();

    iced::application("", IcedExampleView::update, IcedExampleView::view)
        .font(NOTO_SANS_SC)
        .default_font(FONT)
        .theme(IcedExampleView::theme)
        .run()
}

#[derive(Default)]
struct IcedExampleView {
    button_state: ButtonState,
    current_language: Languages,
}

#[derive(Clone, Debug)]
enum Message {
    StateButtonHovered(bool),
    StateButtonPressed,
    StateButtonReleased,
    ToggleLanguage,
}

impl IcedExampleView {
    fn update(&mut self, message: Message) {
        match message {
            Message::StateButtonHovered(hovered) => {
                if hovered {
                    self.button_state = ButtonState::Hovered;
                } else {
                    self.button_state = ButtonState::Normal;
                }
            },
            Message::StateButtonPressed => {
                self.button_state = ButtonState::Pressed;
            },
            Message::StateButtonReleased => {
                self.button_state = ButtonState::Hovered;
            },
            Message::ToggleLanguage => {
                let mut languages: Vec<Languages> = Languages::iter().collect();
                languages.sort_by_key(|a| *a as isize);
                let current_index = languages
                    .iter()
                    .position(|&lang| lang == self.current_language)
                    .unwrap_or(0);
                let next_index = (current_index + 1) % languages.len();
                let next_language = languages[next_index];

                self.current_language = next_language;
                i18n::change_locale(&next_language.into()).unwrap();
            },
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let button_widget = |label| {
            button(text(label).font(FONT).align_x(Center))
                .padding(10)
                .width(150)
        };

        let state_button = mouse_area(button_widget(self.button_state.to_fluent_string()))
            .on_enter(Message::StateButtonHovered(true))
            .on_exit(Message::StateButtonHovered(false))
            .on_press(Message::StateButtonPressed)
            .on_release(Message::StateButtonReleased);

        let language_button = button_widget(
            IcedScreenMessages::ToggleLanguageHint {
                current_language: self.current_language,
            }
            .to_fluent_string(),
        )
        .on_press(Message::ToggleLanguage);

        let content = row![state_button, language_button].spacing(20);

        center(content).into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}
