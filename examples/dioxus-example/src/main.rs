use dioxus::events::Key;
use dioxus::prelude::*;
use es_fluent::{EsFluent, ToFluentString};
use es_fluent_manager_dioxus::*;
use strum::{Display, EnumIter, IntoEnumIterator};
use tracing;
use unic_langid::{LanguageIdentifier, langid};

// Define the i18n module for this crate
es_fluent_manager_dioxus::define_dioxus_i18n_module!();

#[derive(Clone, Copy, Debug, EsFluent, PartialEq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

#[derive(Clone, Copy, Debug, EsFluent)]
pub enum ScreenMessages {
    ToggleLanguageHint { current_language: Languages },
    AppTitle,
    Instructions,
    ButtonDemo,
}

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EsFluent, PartialEq)]
pub enum Languages {
    #[strum(serialize = "en")]
    #[default]
    English,
    #[strum(serialize = "fr")]
    French,
    #[strum(serialize = "cn")]
    Chinese,
}

impl From<Languages> for LanguageIdentifier {
    fn from(val: Languages) -> Self {
        match val {
            Languages::English => langid!("en"),
            Languages::French => langid!("fr"),
            Languages::Chinese => langid!("cn"),
        }
    }
}

fn main() {
    // Enable logging for debugging
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");

    // Set up i18n configuration
    let config = setup_i18n_debug(Languages::default().into());

    // Initialize the i18n system
    init(config);

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div { class: "min-h-screen bg-gray-900 text-white font-sans",
            MainContent {}
        }
    }
}

#[component]
fn MainContent() -> Element {
    let mut current_language = use_signal(|| Languages::default());
    let mut is_loading = use_signal(|| true);

    // Check if assets are loaded
    use_effect(move || {
        spawn(async move {
            loop {
                let loaded = is_language_loaded(&current_language().into());
                is_loading.set(!loaded);
                if loaded {
                    tracing::info!("Assets loaded for language: {:?}", current_language());
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(100).await;
            }
        });
    });

    // Handle keyboard events for language switching
    let handle_keydown = move |event: KeyboardEvent| {
        if event.key() == Key::Character("t".to_string())
            || event.key() == Key::Character("T".to_string())
        {
            let languages: Vec<Languages> = Languages::iter().collect();
            let current_index = languages
                .iter()
                .position(|&lang| lang == current_language())
                .unwrap_or(0);
            let next_index = (current_index + 1) % languages.len();
            let next_language = languages[next_index];

            current_language.set(next_language);
            set_language(next_language.into());
            tracing::info!("Language changed to: {:?}", next_language);
        }
    };

    rsx! {
        div {
            class: "min-h-screen flex flex-col items-center justify-center p-8",
            onkeydown: handle_keydown,
            tabindex: "0",
            autofocus: true,

            if is_loading() {
                LoadingScreen {}
            } else {
                MainUI { current_language }
            }
        }
    }
}

#[component]
fn LoadingScreen() -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center space-y-4",
            div { class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-400" }
            p { class: "text-xl text-gray-300", "Loading translations..." }
        }
    }
}

#[component]
fn MainUI(current_language: Signal<Languages>) -> Element {
    let app_title = ScreenMessages::AppTitle.to_fluent_string();
    let instructions = ScreenMessages::Instructions.to_fluent_string();
    let language_hint = ScreenMessages::ToggleLanguageHint {
        current_language: current_language(),
    }
    .to_fluent_string();
    let current_lang_display = format!("{}", current_language());

    rsx! {
        div { class: "flex flex-col items-center space-y-8 max-w-md mx-auto text-center",
            div { class: "space-y-4",
                h1 {
                    class: "text-4xl font-bold text-blue-400 mb-2",
                    "{app_title}"
                }

                p {
                    class: "text-lg text-gray-300",
                    "{instructions}"
                }
            }

            InteractiveButton {}

            p {
                class: "text-sm text-gray-400 mt-6 px-4 py-2 bg-gray-800 rounded-lg border border-gray-700",
                "{language_hint}"
            }

            div { class: "flex items-center space-x-2 mt-4",
                span { class: "text-sm text-gray-500", "Current:" }
                span {
                    class: "px-3 py-1 bg-blue-600 rounded-full text-sm font-medium",
                    "{current_lang_display}"
                }
            }
        }
    }
}

#[component]
fn InteractiveButton() -> Element {
    let mut button_state = use_signal(|| ButtonState::Normal);

    let button_classes = match button_state() {
        ButtonState::Normal => "bg-gray-700 hover:bg-gray-600 border-gray-500",
        ButtonState::Hovered => "bg-gray-600 border-white",
        ButtonState::Pressed => "bg-green-600 border-red-500",
    };

    let button_demo_title = ScreenMessages::ButtonDemo.to_fluent_string();
    let button_text = button_state().to_fluent_string();

    rsx! {
        div { class: "space-y-4",
            h2 {
                class: "text-xl font-semibold text-gray-200",
                "{button_demo_title}"
            }

            button {
                class: "px-8 py-4 rounded-lg border-2 transition-all duration-200 text-lg font-medium min-w-[150px] {button_classes}",
                onmouseenter: move |_| button_state.set(ButtonState::Hovered),
                onmouseleave: move |_| button_state.set(ButtonState::Normal),
                onmousedown: move |_| button_state.set(ButtonState::Pressed),
                onmouseup: move |_| button_state.set(ButtonState::Hovered),

                "{button_text}"
            }

            p {
                class: "text-sm text-gray-400",
                "Hover, click, and interact with the button above"
            }
        }
    }
}
