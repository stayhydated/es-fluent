use dioxus::prelude::*;
use es_fluent::{EsFluent, ToFluentString as _};
use es_fluent_manager_dioxus::{I18nProvider, use_i18n};
use es_fluent_manager_dioxus_derive::i18n_subscription;
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
    example_shared_lib::force_link();

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
    example_shared_lib::force_link();

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

#[i18n_subscription]
#[component]
fn ClientPreviewBody(initial_language: Languages) -> Element {
    let i18n = match use_i18n() {
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

    let heading = DioxusScreenMessages::ClientHeading.to_fluent_string();
    let summary = DioxusScreenMessages::ClientSummary {
        current_language,
        button_state,
    }
    .to_fluent_string();
    let button_label = DioxusScreenMessages::ClientButtonLabel { next_language }.to_fluent_string();
    let runtime_note = DioxusScreenMessages::RuntimeSplitNote.to_fluent_string();

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

#[i18n_subscription]
#[component]
fn ClientSharedValues(current_language: Languages, button_state: ButtonState) -> Element {
    match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { "Failed to read i18n context: {error}" } },
    };
    let shared_heading = DioxusScreenMessages::SharedTypesHeading.to_fluent_string();
    let shared_language =
        DioxusScreenMessages::SharedLanguageValue { current_language }.to_fluent_string();
    let shared_button_state =
        DioxusScreenMessages::SharedButtonStateValue { button_state }.to_fluent_string();

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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn client_preview_renders_a_localized_snapshot() {
        let html = render_client_preview(Languages::ZhCn);

        assert!(html.contains("客户端 Hook 桥接"));
        assert!(html.contains("切换到"));
        assert!(html.contains("共享值"));
    }
}
