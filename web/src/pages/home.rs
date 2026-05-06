use crate::components::{FeatureCard, FooterPanel, PageHeader, PageLink};
use crate::site::i18n::{HomeHeroMessage, HomeWorkflowMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;

#[component]
pub(crate) fn HomePage(locale: SiteLanguage) -> Element {
    let hero_style = crate::components::use_reveal_style(0, 24.0);
    let workflow_style = crate::components::use_reveal_style(90, 18.0);
    let first_card_style = crate::components::use_reveal_style(160, 16.0);
    let second_card_style = crate::components::use_reveal_style(230, 16.0);
    let third_card_style = crate::components::use_reveal_style(300, 16.0);
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "page-shell", "failed: {error}" } },
    };
    let hero_eyebrow = i18n.localize_message(&HomeHeroMessage::Eyebrow);
    let hero_title = i18n.localize_message(&HomeHeroMessage::Title);
    let hero_body = i18n.localize_message(&HomeHeroMessage::Body);
    let hero_primary_action = i18n.localize_message(&HomeHeroMessage::PrimaryAction);
    let hero_secondary_action = i18n.localize_message(&HomeHeroMessage::SecondaryAction);
    let hero_panel_label = i18n.localize_message(&HomeHeroMessage::PanelLabel);
    let hero_panel_one_title = i18n.localize_message(&HomeHeroMessage::PanelOneTitle);
    let hero_panel_one_body = i18n.localize_message(&HomeHeroMessage::PanelOneBody);
    let hero_panel_two_title = i18n.localize_message(&HomeHeroMessage::PanelTwoTitle);
    let hero_panel_two_body = i18n.localize_message(&HomeHeroMessage::PanelTwoBody);
    let hero_panel_three_title = i18n.localize_message(&HomeHeroMessage::PanelThreeTitle);
    let hero_panel_three_body = i18n.localize_message(&HomeHeroMessage::PanelThreeBody);
    let workflow_title = i18n.localize_message(&HomeWorkflowMessage::Title);
    let workflow_lead = i18n.localize_message(&HomeWorkflowMessage::Lead);
    let workflow_one_title = i18n.localize_message(&HomeWorkflowMessage::OneTitle);
    let workflow_one_body = i18n.localize_message(&HomeWorkflowMessage::OneBody);
    let workflow_two_title = i18n.localize_message(&HomeWorkflowMessage::TwoTitle);
    let workflow_two_body = i18n.localize_message(&HomeWorkflowMessage::TwoBody);
    let workflow_three_title = i18n.localize_message(&HomeWorkflowMessage::ThreeTitle);
    let workflow_three_body = i18n.localize_message(&HomeWorkflowMessage::ThreeBody);

    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Home }
            main { class: "stack",
                section {
                    class: "hero motion-reveal",
                    style: hero_style,
                    div {
                        div { class: "eyebrow", "{hero_eyebrow}" }
                        h1 { "{hero_title}" }
                        p { "{hero_body}" }
                        div { class: "hero-actions",
                            a { class: "button-link primary", href: crate::site::routing::book_href(), "{hero_primary_action}" }
                            PageLink {
                                locale,
                                page: PageKind::Demos,
                                class: "button-link secondary".to_string(),
                                label: hero_secondary_action,
                            }
                        }
                    }
                    aside { class: "hero-panel",
                        h2 { class: "panel-label", "{hero_panel_label}" }
                        ul { class: "hero-list",
                            li {
                                strong { "{hero_panel_one_title}" }
                                span { class: "feature-copy", "{hero_panel_one_body}" }
                            }
                            li {
                                strong { "{hero_panel_two_title}" }
                                span { class: "feature-copy", "{hero_panel_two_body}" }
                            }
                            li {
                                strong { "{hero_panel_three_title}" }
                                span { class: "feature-copy", "{hero_panel_three_body}" }
                            }
                        }
                    }
                }
                section {
                    class: "section-panel motion-reveal",
                    style: workflow_style,
                    h2 { class: "section-title", "{workflow_title}" }
                    p { class: "section-lead", "{workflow_lead}" }
                    div { class: "grid columns-3",
                        FeatureCard {
                            title: workflow_one_title,
                            body: workflow_one_body,
                            style: first_card_style,
                        }
                        FeatureCard {
                            title: workflow_two_title,
                            body: workflow_two_body,
                            style: second_card_style,
                        }
                        FeatureCard {
                            title: workflow_three_title,
                            body: workflow_three_body,
                            style: third_card_style,
                        }
                    }
                }
            }
            FooterPanel {}
        }
    }
}
