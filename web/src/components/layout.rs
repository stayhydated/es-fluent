use crate::components::PageLink;
use crate::site::constants::ES_FLUENT_MANAGER_DIOXUS_CRATES_URL;
use crate::site::i18n::{SiteChromeMessage, SiteFooterMessage, SiteLanguage};
use crate::site::routing::{PageKind, book_href, page_href};
use dioxus::prelude::*;
use es_fluent::ToFluentString as _;
use es_fluent_manager_dioxus_derive::i18n_subscription;

#[i18n_subscription]
#[component]
pub(crate) fn PageHeader(locale: SiteLanguage, current_page: PageKind) -> Element {
    rsx! {
        header { class: "page-header",
            a { class: "brand", href: page_href(locale, PageKind::Home),
                    span { class: "brand-mark", "EF" }
                    span { class: "brand-copy",
                        span { class: "brand-kicker", "{SiteChromeMessage::PageKicker.to_fluent_string()}" }
                        span { class: "brand-title", "{SiteChromeMessage::SiteName.to_fluent_string()}" }
                    }
                }
                div { class: "header-cluster",
                nav { class: "nav-pill",
                    PageLink {
                        locale,
                        page: PageKind::Home,
                        class: if current_page == PageKind::Home { " nav-link is-active".to_string() } else { " nav-link".to_string() },
                        label: SiteChromeMessage::NavHome.to_fluent_string(),
                    }
                    PageLink {
                        locale,
                        page: PageKind::Demos,
                        class: if current_page == PageKind::Demos || current_page == PageKind::Bevy { " nav-link is-active".to_string() } else { " nav-link".to_string() },
                        label: SiteChromeMessage::NavDemos.to_fluent_string(),
                    }
                    ExternalNavLink {
                        href: book_href(),
                        label: SiteChromeMessage::NavDocs.to_fluent_string(),
                    }
                    ExternalNavLink {
                        href: "https://github.com/stayhydated/es-fluent".to_string(),
                        label: SiteChromeMessage::NavSource.to_fluent_string(),
                    }
                }
                LocaleSwitcher { locale, current_page }
            }
        }
    }
}

#[component]
fn ExternalNavLink(href: String, label: String) -> Element {
    let is_external = href.starts_with("http");

    rsx! {
        a {
            class: " nav-link",
            href,
            target: if is_external { Some("_blank") } else { None },
            rel: if is_external { Some("noreferrer") } else { None },
            "{label}"
        }
    }
}

#[i18n_subscription]
#[component]
fn LocaleSwitcher(locale: SiteLanguage, current_page: PageKind) -> Element {
    rsx! {
        div { class: "locale-switcher",
            span { class: "locale-label", "{SiteChromeMessage::LocaleLabel.to_fluent_string()}" }
            for candidate in SiteLanguage::all() {
                PageLink {
                    locale: candidate,
                    page: current_page,
                    class: if candidate == locale { "locale-link is-active".to_string() } else { "locale-link".to_string() },
                    label: candidate.display_label(),
                }
            }
        }
    }
}

#[i18n_subscription]
#[component]
pub(crate) fn FooterPanel() -> Element {
    rsx! {
        footer { class: "site-footer",
            p { class: "footer-copy",
                span { class: "footer-label", "{SiteFooterMessage::Label.to_fluent_string()}" }
                span { class: "footer-text",
                    "{SiteFooterMessage::BodyPrefix.to_fluent_string()} "
                    a {
                        class: "footer-link",
                        href: ES_FLUENT_MANAGER_DIOXUS_CRATES_URL,
                        target: "_blank",
                        rel: "noreferrer",
                        "{SiteFooterMessage::BodyLinkLabel.to_fluent_string()}"
                    }
                    "."
                }
            }
        }
    }
}
