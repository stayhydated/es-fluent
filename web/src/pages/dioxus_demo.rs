use crate::pages::i18n::{
    CourtyardLocaleStatus, CourtyardWelcome, DemoLanguage, EmberEchoCopy, PiesAtDusk,
};
use crate::site::{
    constants::{PROJECT, VERSION},
    routing::PageKind,
};
use dioxus::prelude::*;
use es_fluent_manager_dioxus::{DioxusAssetI18nProvider, use_i18n};
use stayhydated_dioxus::{
    NavigationTarget, StayhydatedProjectPortalShell, select, surface_reveal_style,
};
use strum::IntoEnumIterator as _;

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
    let courtyard_style = surface_reveal_style();
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => {
            return rsx! {
                StayhydatedProjectPortalShell {
                    project: PROJECT,
                    version: VERSION,
                    home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
                    div { class: "demo-page courtyard-page", lang: "en-US",
                        section { class: "courtyard-closed section-band",
                            span { class: "panel-label", "Ember & Echo · oven closed" }
                            h1 { "The courtyard fire could not be kindled" }
                            p { "The oven doors are closed while the localization provider recovers." }
                            code { class: "courtyard-diagnostic", "{error}" }
                        }
                    }
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

    let restaurant_name = i18n.localize_message(&EmberEchoCopy::RestaurantName);
    let kicker = i18n.localize_message(&EmberEchoCopy::Kicker);
    let title = i18n.localize_message(&EmberEchoCopy::Title);
    let manifesto = i18n.localize_message(&EmberEchoCopy::Manifesto);
    let signature_name = i18n.localize_message(&EmberEchoCopy::SignatureName);
    let signature_description = i18n.localize_message(&EmberEchoCopy::SignatureDescription);
    let rivalry_claim = i18n.localize_message(&EmberEchoCopy::RivalryClaim);
    let select_label = i18n.localize_message(&EmberEchoCopy::SelectLabel);
    let image_alt = i18n.localize_message(&EmberEchoCopy::ImageAlt);
    let dough_title = i18n.localize_message(&EmberEchoCopy::DoughTitle);
    let dough_body = i18n.localize_message(&EmberEchoCopy::DoughBody);
    let fire_title = i18n.localize_message(&EmberEchoCopy::FireTitle);
    let fire_body = i18n.localize_message(&EmberEchoCopy::FireBody);
    let finish_title = i18n.localize_message(&EmberEchoCopy::FinishTitle);
    let finish_body = i18n.localize_message(&EmberEchoCopy::FinishBody);
    let locale_status = i18n.localize_message(&CourtyardLocaleStatus {
        locale: &selected_label,
    });
    let welcome = i18n.localize_message(&CourtyardWelcome { guest: "Sorrel" });
    let pies = i18n.localize_message(&PiesAtDusk { count: 3 });
    let page_output_dir = crate::site::routing::output_dir(PageKind::Dioxus);
    let site_root = crate::site::routing::site_root_prefix(&page_output_dir);
    let hero_src = format!("{site_root}assets/pizzerias/ember-and-echo.webp");
    let document_language = i18n.requested_language().to_string();

    rsx! {
        StayhydatedProjectPortalShell {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            div { class: "demo-page courtyard-page", lang: document_language,
                section {
                    class: "ember-courtyard motion-reveal",
                    style: courtyard_style.as_str(),
                    header { class: "courtyard-header",
                        div { class: "courtyard-heading",
                            span { class: "panel-label", "{kicker}" }
                            p { class: "courtyard-brand", "{restaurant_name}" }
                            h1 { "{title}" }
                            p { class: "courtyard-manifesto", "{manifesto}" }
                        }
                        div { class: "courtyard-language",
                            select::Select::<DemoLanguage> {
                                value: Some(selected.into()),
                                on_value_change: on_change,
                                select::SelectTrigger {
                                    aria_label: select_label.clone(),
                                    select::SelectValue { placeholder: selected_label }
                                }
                                select::SelectList { aria_label: select_label,
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
                            p { class: "courtyard-locale-status", "{locale_status}" }
                        }
                    }
                    div { class: "courtyard-menu-grid",
                        figure { class: "courtyard-photo",
                            img {
                                src: hero_src,
                                alt: image_alt,
                                width: "1600",
                                height: "2400",
                                decoding: "async",
                                fetchpriority: "high",
                            }
                            figcaption {
                                span { "{signature_name}" }
                                strong { "{signature_description}" }
                            }
                        }
                        aside { class: "courtyard-order", aria_live: "polite",
                            p { class: "courtyard-call", "{welcome}" }
                            p { class: "courtyard-count", "{pies}" }
                            blockquote { "{rivalry_claim}" }
                        }
                    }
                    div { class: "courtyard-proof",
                        article { class: "courtyard-step dough-step",
                            span { class: "step-number", "I" }
                            h2 { "{dough_title}" }
                            p { "{dough_body}" }
                        }
                        article { class: "courtyard-step fire-step",
                            span { class: "step-number", "II" }
                            h2 { "{fire_title}" }
                            p { "{fire_body}" }
                        }
                        article { class: "courtyard-step finish-step",
                            span { class: "step-number", "III" }
                            h2 { "{finish_title}" }
                            p { "{finish_body}" }
                        }
                    }
                }
            }
        }
    }
}
