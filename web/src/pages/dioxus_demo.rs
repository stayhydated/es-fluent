use crate::components::{FooterPanel, PageHeader};
use crate::pages::i18n::{DemoLanguage, DioxusDemoMessage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_i18n};
use stayhydated_dioxus::{ProjectPageShell, select, surface_reveal_style};
use strum::IntoEnumIterator as _;

const OSMOSE_IMAGE_URL: &str = "https://www.expressivee.com/img/products/osmose/osmose2.png";

#[component]
pub(crate) fn DioxusPage() -> Element {
    rsx! {
        DioxusAssetI18nProvider {
            initial_language: DemoLanguage::default().into(),
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

    let selected = use_memo({
        let i18n = i18n.clone();
        move || Some(DemoLanguage::try_from(i18n.requested_language()).unwrap_or_default())
    });
    let selected_label = i18n.localize_message(&selected().unwrap_or_default());
    let options = DemoLanguage::iter()
        .map(|language| (language, i18n.localize_message(&language)))
        .collect::<Vec<_>>();
    let i18n_for_select = i18n.clone();
    let on_change = move |next_language: Option<DemoLanguage>| {
        let Some(next_language) = next_language else {
            return;
        };

        let _ = i18n_for_select.select_language(next_language);
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
                        select::Select::<DemoLanguage> {
                            value: Some(selected.into()),
                            on_value_change: on_change,
                            select::SelectTrigger {
                                aria_label: "Language",
                                select::SelectValue { placeholder: selected_label }
                            }
                            select::SelectList { aria_label: "Languages",
                                for (index, (language, label)) in options.iter().enumerate() {
                                    {
                                        let active = Some(*language) == selected();
                                        rsx! {
                                            select::SelectOption::<DemoLanguage> {
                                                key: "{language:?}",
                                                index,
                                                value: *language,
                                                text_value: Some(label.clone()),
                                                "{label}"
                                                if active {
                                                    select::SelectItemIndicator {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
