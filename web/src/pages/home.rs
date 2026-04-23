use crate::components::{FeatureCard, FooterPanel, PageHeader, PageLink};
use crate::site::i18n::{HomeHeroMessage, HomeWorkflowMessage, SiteLanguage};
use crate::site::routing::{PageKind, book_href};
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent::ToFluentString as _;

#[component]
pub(crate) fn HomePage(locale: SiteLanguage) -> Element {
    rsx! {
        div { class: "page-shell",
            PageHeader { locale, current_page: PageKind::Home }
            main { class: "stack",
                section { class: "hero",
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
                section { class: "section-panel",
                    h2 { class: "section-title", "{HomeWorkflowMessage::Title.to_fluent_string()}" }
                    p { class: "section-lead", "{HomeWorkflowMessage::Lead.to_fluent_string()}" }
                    div { class: "grid columns-3",
                        FeatureCard {
                            title: HomeWorkflowMessage::OneTitle.to_fluent_string(),
                            body: HomeWorkflowMessage::OneBody.to_fluent_string(),
                        }
                        FeatureCard {
                            title: HomeWorkflowMessage::TwoTitle.to_fluent_string(),
                            body: HomeWorkflowMessage::TwoBody.to_fluent_string(),
                        }
                        FeatureCard {
                            title: HomeWorkflowMessage::ThreeTitle.to_fluent_string(),
                            body: HomeWorkflowMessage::ThreeBody.to_fluent_string(),
                        }
                    }
                }
            }
            FooterPanel {}
        }
    }
}
