use dioxus_core::{Element, VirtualDom};
use dioxus_core_macro::{Props, component, rsx};
use dioxus_hooks::use_signal;
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use dioxus_signals::WritableExt as _;
use es_fluent::{EsFluent, ToFluentString as _};
use es_fluent_manager_dioxus::{
    GlobalLocalizerMode,
    desktop::{use_global_localized, use_init_i18n_with_mode},
    ssr::SsrI18n,
};
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

    for language in Languages::iter() {
        let client_html = render_client_preview(language);
        let ssr_html = render_ssr_preview(language);
        let tag = language_tag(language);

        output.push_str("=== ");
        output.push_str(&tag);
        output.push_str(" ===\n");
        output.push_str("[client desktop/mobile/web]\n");
        output.push_str(&client_html);
        output.push_str("\n\n");
        output.push_str("[ssr]\n");
        output.push_str(&ssr_html);
        output.push_str("\n\n");
    }

    output
}

pub fn render_client_preview(initial_language: Languages) -> String {
    example_shared_lib::force_link();

    let mut dom =
        VirtualDom::new_with_props(ClientPreview, ClientPreviewProps { initial_language });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

pub fn render_ssr_preview(initial_language: Languages) -> String {
    example_shared_lib::force_link();

    let i18n = SsrI18n::try_new_with_discovered_modules_and_mode(
        initial_language,
        GlobalLocalizerMode::ReplaceExisting,
    )
    .expect("Dioxus SSR example should initialize");

    let mut dom = VirtualDom::new_with_props(SsrPreview, SsrPreviewProps { initial_language });

    i18n.rebuild_and_render(&mut dom)
}

#[component]
fn ClientPreview(initial_language: Languages) -> Element {
    let i18n = use_init_i18n_with_mode(initial_language, GlobalLocalizerMode::ReplaceExisting);
    let mut is_hovered = use_signal(|| false);

    let current_language =
        Languages::try_from(&i18n.requested_language()).unwrap_or(initial_language);
    let button_state = if is_hovered() {
        ButtonState::Hovered
    } else {
        ButtonState::Normal
    };
    let next_language = current_language.next();

    let heading = i18n.localize_global_fluent(&DioxusScreenMessages::ClientHeading);
    let summary = i18n.localize_global_fluent(&DioxusScreenMessages::ClientSummary {
        current_language,
        button_state,
    });
    let button_label =
        i18n.localize_global_fluent(&DioxusScreenMessages::ClientButtonLabel { next_language });
    let runtime_note = i18n.localize_global_fluent(&DioxusScreenMessages::RuntimeSplitNote);

    rsx! {
        section {
            class: "dioxus-preview client-preview",
            "data-runtime": "desktop/mobile/web",
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
    let shared_heading = use_global_localized(&DioxusScreenMessages::SharedTypesHeading);
    let shared_language =
        use_global_localized(&DioxusScreenMessages::SharedLanguageValue { current_language });
    let shared_button_state =
        use_global_localized(&DioxusScreenMessages::SharedButtonStateValue { button_state });

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

#[component]
fn SsrPreview(initial_language: Languages) -> Element {
    let button_state = ButtonState::Pressed;
    let heading = DioxusScreenMessages::SsrHeading.to_fluent_string();
    let summary = DioxusScreenMessages::SsrSummary {
        current_language: initial_language,
        button_state,
    }
    .to_fluent_string();
    let shared_heading = DioxusScreenMessages::SharedTypesHeading.to_fluent_string();
    let shared_language = DioxusScreenMessages::SharedLanguageValue {
        current_language: initial_language,
    }
    .to_fluent_string();
    let shared_button_state =
        DioxusScreenMessages::SharedButtonStateValue { button_state }.to_fluent_string();

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

    #[test]
    #[serial]
    fn ssr_preview_renders_a_localized_snapshot() {
        let html = render_ssr_preview(Languages::FrFr);

        assert!(html.contains("Pont SSR"));
        assert!(html.contains("Langue active"));
        assert!(html.contains("État du bouton"));
    }
}
