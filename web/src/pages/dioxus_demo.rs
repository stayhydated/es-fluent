use crate::components::{FooterPanel, PageHeader};
use crate::pages::i18n::{DemoLanguage, DioxusDemoMessage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_i18n};
use stayhydated_dioxus::{
    LanguageSelect, ProjectPageShell, StayhydatedSiteLanguage as _,
    stayhydated_all_language_options, stayhydated_selected_language_or_default,
    surface_reveal_style,
};

const OSMOSE_IMAGE_URL: &str = "https://www.expressivee.com/img/products/osmose/osmose2.png";

#[component]
pub(crate) fn DioxusPage() -> Element {
    rsx! {
        DioxusAssetI18nProvider {
            initial_language: DemoLanguage::default().language_identifier(),
            DioxusDemoContent {}
        }
    }
}

#[component]
fn DioxusDemoContent() -> Element {
    let demo_style = surface_reveal_style();
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => {
            return rsx! {
                div { class: "page-shell",
                    PageHeader { current_page: PageKind::Dioxus }
                    main { class: "stack",
                        section { class: "page-title-band",
                            span { class: "panel-label", "demo load failure" }
                            h1 { "Expressive E Osmose" }
                            p { "Failed to initialize the Osmose demo: {error}" }
                        }
                    }
                    FooterPanel {}
                }
            };
        },
    };

    let selected =
        stayhydated_selected_language_or_default::<DemoLanguage>(i18n.requested_language());
    let options = stayhydated_all_language_options::<DemoLanguage>(|language| {
        i18n.localize_message(&language)
    });
    let i18n_for_select = i18n.clone();
    let on_change = move |next_language: DemoLanguage| {
        let _ = i18n_for_select.select_language(next_language.language_identifier());
    };

    let panel_label = i18n.localize_message(&DioxusDemoMessage::PanelLabel);
    let title = i18n.localize_message(&DioxusDemoMessage::Title);
    let body = i18n.localize_message(&DioxusDemoMessage::Body);
    let result_label = i18n.localize_message(&DioxusDemoMessage::ResultLabel);
    let result_body = i18n.localize_message(&DioxusDemoMessage::ResultBody);
    let runtime_title = i18n.localize_message(&DioxusDemoMessage::RuntimeTitle);
    let runtime_body = i18n.localize_message(&DioxusDemoMessage::RuntimeBody);
    let resource_title = i18n.localize_message(&DioxusDemoMessage::ResourceTitle);
    let resource_body = i18n.localize_message(&DioxusDemoMessage::ResourceBody);

    rsx! {
        ProjectPageShell {
            header: rsx!(PageHeader { current_page: PageKind::Dioxus }),
            footer: Some(rsx!(FooterPanel {})),
            section { class: "dioxus-keyboard-hero motion-reveal",
                style: demo_style.as_str(),
                div { class: "dioxus-demo-card-header",
                    div { class: "dioxus-demo-card-heading",
                        span { class: "panel-label", "{panel_label}" }
                        h1 { "{title}" }
                        p { "{body}" }
                    }
                    div { class: "dioxus-demo-card-controls",
                        LanguageSelect::<DemoLanguage> {
                            label: "Language",
                            selected,
                            options,
                            on_change,
                        }
                    }
                }
                div {
                    class: "osmose-visual",
                    div { class: "osmose-panel",
                        span { "Expressive E" }
                        strong { "Osmose" }
                    }
                    div { class: "osmose-image-frame",
                        img {
                            class: "osmose-product-image",
                            src: OSMOSE_IMAGE_URL,
                            alt: "Expressive E Osmose 49/61-key synthesizer",
                        }
                    }
                }
                div { class: "dioxus-keyboard-proof",
                    article { class: "feature-card",
                        span { class: "panel-label", "{result_label}" }
                        p { class: "feature-copy", "{result_body}" }
                    }
                    article { class: "feature-card",
                        h2 { class: "feature-title", "{runtime_title}" }
                        p { class: "feature-copy", "{runtime_body}" }
                    }
                    article { class: "feature-card",
                        h2 { class: "feature-title", "{resource_title}" }
                        p { class: "feature-copy", "{resource_body}" }
                    }
                }
            }
        }
    }
}
