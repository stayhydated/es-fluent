use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::{Props, component, rsx};
use dioxus_hooks::use_signal;
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use dioxus_signals::WritableExt as _;
use es_fluent::{EsFluent, FluentValue};
use es_fluent_manager_dioxus::{ManagedI18n, use_i18n, use_provide_i18n};
use example_shared_lib::{ButtonState, Languages};
use std::collections::HashMap;
use strum::IntoEnumIterator as _;
use unic_langid::LanguageIdentifier;

es_fluent_manager_dioxus::define_i18n_module!();

const DOMAIN: &str = env!("CARGO_PKG_NAME");
const CLIENT_HEADING: &str = "dioxus_screen_messages-ClientHeading";
const CLIENT_SUMMARY: &str = "dioxus_screen_messages-ClientSummary";
const CLIENT_BUTTON_LABEL: &str = "dioxus_screen_messages-ClientButtonLabel";
const RUNTIME_SPLIT_NOTE: &str = "dioxus_screen_messages-RuntimeSplitNote";
const SHARED_TYPES_HEADING: &str = "dioxus_screen_messages-SharedTypesHeading";
const SHARED_LANGUAGE_VALUE: &str = "dioxus_screen_messages-SharedLanguageValue";
const SHARED_BUTTON_STATE_VALUE: &str = "dioxus_screen_messages-SharedButtonStateValue";

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

    let managed = ManagedI18n::try_new_with_discovered_modules(initial_language)
        .expect("Dioxus client example manager should initialize");
    let mut dom = VirtualDom::new_with_props(
        ClientPreview,
        ClientPreviewProps {
            initial_language,
            managed,
        },
    );
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[component]
fn ClientPreview(initial_language: Languages, managed: ManagedI18n) -> Element {
    let i18n = use_provide_i18n(managed);
    let mut is_hovered = use_signal(|| false);

    let current_language =
        Languages::try_from(&i18n.requested_language()).unwrap_or(initial_language);
    let button_state = if is_hovered() {
        ButtonState::Hovered
    } else {
        ButtonState::Normal
    };
    let next_language = current_language.next();

    let heading = i18n.localize_in_domain(DOMAIN, CLIENT_HEADING, None);
    let summary_args = fluent_args([
        ("current_language", language_tag(current_language)),
        ("button_state", format!("{button_state:?}")),
    ]);
    let summary = i18n.localize_in_domain(DOMAIN, CLIENT_SUMMARY, Some(&summary_args));
    let button_args = fluent_args([("next_language", language_tag(next_language))]);
    let button_label = i18n.localize_in_domain(DOMAIN, CLIENT_BUTTON_LABEL, Some(&button_args));
    let runtime_note = i18n.localize_in_domain(DOMAIN, RUNTIME_SPLIT_NOTE, None);

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
                    if let Err(error) = i18n.try_select_language(next_language) {
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
    let i18n = use_i18n();
    let shared_heading = i18n.localize_in_domain(DOMAIN, SHARED_TYPES_HEADING, None);
    let language_args = fluent_args([("current_language", language_tag(current_language))]);
    let shared_language =
        i18n.localize_in_domain(DOMAIN, SHARED_LANGUAGE_VALUE, Some(&language_args));
    let button_args = fluent_args([("button_state", format!("{button_state:?}"))]);
    let shared_button_state =
        i18n.localize_in_domain(DOMAIN, SHARED_BUTTON_STATE_VALUE, Some(&button_args));

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

fn fluent_args<const N: usize>(
    items: [(&'static str, String); N],
) -> HashMap<&'static str, FluentValue<'static>> {
    items
        .into_iter()
        .map(|(key, value)| (key, FluentValue::from(value)))
        .collect()
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
