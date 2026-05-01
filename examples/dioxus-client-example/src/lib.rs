use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::I18nProvider;
use example_shared_lib::{ButtonState, Languages};
use strum::IntoEnumIterator as _;
use unic_langid::LanguageIdentifier;

es_fluent_manager_dioxus::define_i18n_module!();

#[derive(Clone, Copy, Debug, EsFluent)]
#[fluent(namespace = "ui")]
pub enum DioxusScreenMessages {
    ClientHeading,
    ClientSummary {
        current_language: Languages,
        button_state: ButtonState,
    },
    ClientButtonLabel {
        next_language: Languages,
    },
    RuntimeSplitNote,
    SharedTypesHeading,
    SharedLanguageValue {
        current_language: Languages,
    },
    SharedButtonStateValue {
        button_state: ButtonState,
    },
    SsrHeading,
    SsrSummary {
        current_language: Languages,
        button_state: ButtonState,
    },
}

pub fn render_showcase() -> String {
    let mut output = String::new();

    let language = Languages::iter()
        .next()
        .expect("Dioxus client example should have at least one language");
    let client_html = render_client_preview(language);
    let tag = language_tag(language);

    output.push_str("=== ");
    output.push_str(&tag);
    output.push_str(" ===\n");
    output.push_str("[client]\n");
    output.push_str(&client_html);
    output.push_str("\n\n");

    output
}

pub fn render_client_preview(initial_language: Languages) -> String {
    let mut dom =
        VirtualDom::new_with_props(ClientPreview, ClientPreviewProps { initial_language });
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

#[component]
fn ClientPreview(initial_language: Languages) -> Element {
    rsx! {
        I18nProvider {
            initial_language: LanguageIdentifier::from(initial_language),
            ClientPreviewBody { initial_language }
        }
    }
}

#[component]
fn ClientPreviewBody(initial_language: Languages) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { section { "Failed to initialize i18n: {error}" } },
    };
    let mut is_hovered = use_signal(|| false);

    let current_language =
        Languages::try_from(&i18n.requested_language()).unwrap_or(initial_language);
    let button_state = if is_hovered() {
        ButtonState::Hovered
    } else {
        ButtonState::Normal
    };
    let next_language = current_language.next();

    let heading = i18n.localize_message(&DioxusScreenMessages::ClientHeading);
    let summary = i18n.localize_message(&DioxusScreenMessages::ClientSummary {
        current_language,
        button_state,
    });
    let button_label =
        i18n.localize_message(&DioxusScreenMessages::ClientButtonLabel { next_language });
    let runtime_note = i18n.localize_message(&DioxusScreenMessages::RuntimeSplitNote);

    rsx! {
        section {
            class: "dioxus-preview client-preview",
            "data-runtime": "client",
            h1 { "{heading}" }
            p { "{summary}" }
            p { "{runtime_note}" }
            ClientSharedValues { current_language, button_state }
            button {
                r#type: "button",
                onclick: move |_| {
                    is_hovered.set(!is_hovered());
                    if let Err(error) = i18n.select_language(next_language) {
                        eprintln!("example locale switch failed: {error}");
                    }
                },
                "{button_label}"
            }
        }
    }
}

#[component]
fn ClientSharedValues(current_language: Languages, button_state: ButtonState) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { "Failed to read i18n context: {error}" } },
    };
    let shared_heading = i18n.localize_message(&DioxusScreenMessages::SharedTypesHeading);
    let shared_language =
        i18n.localize_message(&DioxusScreenMessages::SharedLanguageValue { current_language });
    let shared_button_state =
        i18n.localize_message(&DioxusScreenMessages::SharedButtonStateValue { button_state });

    rsx! {
        div {
            class: "shared-values",
            h2 { "{shared_heading}" }
            ul {
                li { "{shared_language}" }
                li { "{shared_button_state}" }
            }
        }
    }
}

fn language_tag(language: Languages) -> String {
    let id: LanguageIdentifier = language.into();
    id.to_string()
}
