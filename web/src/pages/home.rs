use crate::components::{FeatureCard, FooterPanel, PageHeader, PageLink, use_reveal_style};
use crate::site::i18n::{HomeHeroMessage, HomeWorkflowMessage, SiteLanguage};
use crate::site::routing::{PageKind, book_href};
use dioxus::prelude::*;
use es_fluent::ToFluentString as _;

#[component]
pub(crate) fn HomePage(locale: SiteLanguage) -> Element {
    let hero_style = use_reveal_style(0, 24.0);
    let workflow_style = use_reveal_style(90, 18.0);
    let first_card_style = use_reveal_style(160, 16.0);
    let second_card_style = use_reveal_style(230, 16.0);
    let third_card_style = use_reveal_style(300, 16.0);

    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Home }
            main { class: "stack",
                section {
                    class: "hero motion-reveal",
                    style: hero_style,
                    div {
                        div { class: "eyebrow", "{HomeHeroMessage::Eyebrow.to_fluent_string()}" }
                        h1 { "{HomeHeroMessage::Title.to_fluent_string()}" }
                        p { "{HomeHeroMessage::Body.to_fluent_string()}" }
                        div { class: "hero-actions",
                            a { class: "button-link primary", href: book_href(), "{HomeHeroMessage::PrimaryAction.to_fluent_string()}" }
                            PageLink {
                                locale,
                                page: PageKind::Demos,
                                class: "button-link secondary".to_string(),
                                label: HomeHeroMessage::SecondaryAction.to_fluent_string(),
                            }
                        }
                    }
                    aside { class: "hero-panel",
                        h2 { class: "panel-label", "{HomeHeroMessage::PanelLabel.to_fluent_string()}" }
                        ul { class: "hero-list",
                            li {
                                strong { "{HomeHeroMessage::PanelOneTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{HomeHeroMessage::PanelOneBody.to_fluent_string()}" }
                            }
                            li {
                                strong { "{HomeHeroMessage::PanelTwoTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{HomeHeroMessage::PanelTwoBody.to_fluent_string()}" }
                            }
                            li {
                                strong { "{HomeHeroMessage::PanelThreeTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{HomeHeroMessage::PanelThreeBody.to_fluent_string()}" }
                            }
                        }
                    }
                }
                section {
                    class: "section-panel motion-reveal",
                    style: workflow_style,
                    h2 { class: "section-title", "{HomeWorkflowMessage::Title.to_fluent_string()}" }
                    p { class: "section-lead", "{HomeWorkflowMessage::Lead.to_fluent_string()}" }
                    div { class: "grid columns-3",
                        FeatureCard {
                            title: HomeWorkflowMessage::OneTitle.to_fluent_string(),
                            body: HomeWorkflowMessage::OneBody.to_fluent_string(),
                            style: first_card_style,
                        }
                        FeatureCard {
                            title: HomeWorkflowMessage::TwoTitle.to_fluent_string(),
                            body: HomeWorkflowMessage::TwoBody.to_fluent_string(),
                            style: second_card_style,
                        }
                        FeatureCard {
                            title: HomeWorkflowMessage::ThreeTitle.to_fluent_string(),
                            body: HomeWorkflowMessage::ThreeBody.to_fluent_string(),
                            style: third_card_style,
                        }
                    }
                }
            }
            FooterPanel {}
        }
    }
}
