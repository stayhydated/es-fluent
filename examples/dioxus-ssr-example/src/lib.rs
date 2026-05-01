use dioxus::prelude::*;
use es_fluent::EsFluent;
use es_fluent_manager_dioxus::ssr::{SsrI18n, SsrI18nRuntime};
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
    let runtime = SsrI18nRuntime::new();
    let mut output = String::new();

    for language in Languages::iter() {
        let html = render_ssr_preview_with_runtime(&runtime, language);
        let tag = language_tag(language);

        output.push_str("=== ");
        output.push_str(&tag);
        output.push_str(" ===\n");
        output.push_str("[ssr]\n");
        output.push_str(&html);
        output.push_str("\n\n");
    }

    output
}

pub fn render_ssr_preview(initial_language: Languages) -> String {
    let runtime = SsrI18nRuntime::new();
    render_ssr_preview_with_runtime(&runtime, initial_language)
}

fn render_ssr_preview_with_runtime(
    runtime: &SsrI18nRuntime,
    initial_language: Languages,
) -> String {
    let i18n = runtime
        .request(initial_language)
        .expect("Dioxus SSR example should initialize");
    let mut dom = VirtualDom::new_with_props(
        SsrPreview,
        SsrPreviewProps {
            initial_language,
            i18n: i18n.clone(),
        },
    );

    i18n.rebuild_and_render(&mut dom)
}

#[component]
fn SsrPreview(initial_language: Languages, i18n: SsrI18n) -> Element {
    let button_state = ButtonState::Pressed;
    let heading = i18n.localize_message(&DioxusScreenMessages::SsrHeading);
    let summary = i18n.localize_message(&DioxusScreenMessages::SsrSummary {
        current_language: initial_language,
        button_state,
    });
    let shared_heading = i18n.localize_message(&DioxusScreenMessages::SharedTypesHeading);
    let shared_language = i18n.localize_message(&DioxusScreenMessages::SharedLanguageValue {
        current_language: initial_language,
    });
    let shared_button_state =
        i18n.localize_message(&DioxusScreenMessages::SharedButtonStateValue { button_state });

    rsx! {
        section {
            class: "dioxus-preview ssr-preview",
            "data-runtime": "ssr",
            h1 { "{heading}" }
            p { "{summary}" }
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
}

fn language_tag(language: Languages) -> String {
    let id: LanguageIdentifier = language.into();
    id.to_string()
}
