use crate::components::{FeatureCard, FooterPanel, PageHeader, PageLink};
use crate::site::i18n::{SiteLanguage, SiteMessage};
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
                        div { class: "eyebrow", "{SiteMessage::HeroEyebrow.to_fluent_string()}" }
                        h1 { "{SiteMessage::HeroTitle.to_fluent_string()}" }
                        p { "{SiteMessage::HeroBody.to_fluent_string()}" }
                        div { class: "hero-actions",
                            a { class: "button-link primary", href: book_href(), "{SiteMessage::HeroPrimary.to_fluent_string()}" }
                            PageLink {
                                locale,
                                page: PageKind::Demos,
                                class: "button-link secondary".to_string(),
                                label: SiteMessage::HeroSecondary.to_fluent_string(),
                            }
                        }
                    }
                    aside { class: "hero-panel",
                        h2 { class: "panel-label", "{SiteMessage::HeroPanelLabel.to_fluent_string()}" }
                        ul { class: "hero-list",
                            li {
                                strong { "{SiteMessage::HeroPanelOneTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{SiteMessage::HeroPanelOneBody.to_fluent_string()}" }
                            }
                            li {
                                strong { "{SiteMessage::HeroPanelTwoTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{SiteMessage::HeroPanelTwoBody.to_fluent_string()}" }
                            }
                            li {
                                strong { "{SiteMessage::HeroPanelThreeTitle.to_fluent_string()}" }
                                span { class: "feature-copy", "{SiteMessage::HeroPanelThreeBody.to_fluent_string()}" }
                            }
                        }
                    }
                }
                section { class: "section-panel",
                    h2 { class: "section-title", "{SiteMessage::FeatureTitle.to_fluent_string()}" }
                    p { class: "section-lead", "{SiteMessage::FeatureLead.to_fluent_string()}" }
                    div { class: "grid columns-3",
                        FeatureCard {
                            title: SiteMessage::FeatureOneTitle.to_fluent_string(),
                            body: SiteMessage::FeatureOneBody.to_fluent_string(),
                        }
                        FeatureCard {
                            title: SiteMessage::FeatureTwoTitle.to_fluent_string(),
                            body: SiteMessage::FeatureTwoBody.to_fluent_string(),
                        }
                        FeatureCard {
                            title: SiteMessage::FeatureThreeTitle.to_fluent_string(),
                            body: SiteMessage::FeatureThreeBody.to_fluent_string(),
                        }
                    }
                }
            }
            FooterPanel {}
        }
    }
}
