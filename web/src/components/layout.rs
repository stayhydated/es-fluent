use crate::components::PageLink;
use crate::site::constants::ES_FLUENT_MANAGER_DIOXUS_CRATES_URL;
use crate::site::i18n::{SiteChromeMessage, SiteFooterMessage, SiteLanguage};
use crate::site::routing::PageKind;
use dioxus::prelude::*;

#[component]
pub(crate) fn PageHeader(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { header { class: "page-header", "failed: {error}" } },
    };
    let page_kicker = i18n.localize_message(&SiteChromeMessage::PageKicker);
    let site_name = i18n.localize_message(&SiteChromeMessage::SiteName);
    let nav_home = i18n.localize_message(&SiteChromeMessage::NavHome);
    let nav_demos = i18n.localize_message(&SiteChromeMessage::NavDemos);
    let nav_docs = i18n.localize_message(&SiteChromeMessage::NavDocs);
    let nav_source = i18n.localize_message(&SiteChromeMessage::NavSource);

    rsx! {
        header { class: "page-header",
            a { class: "brand", href: crate::site::routing::page_href(locale, PageKind::Home),
                    span { class: "brand-mark", "EF" }
                    span { class: "brand-copy",
                        span { class: "brand-kicker", "{page_kicker}" }
                        span { class: "brand-title", "{site_name}" }
                    }
                }
                div { class: "header-cluster",
                nav { class: "nav-pill",
                    PageLink {
                        locale,
                        page: PageKind::Home,
                        class: if current_page == PageKind::Home { " nav-link is-active".to_string() } else { " nav-link".to_string() },
                        label: nav_home,
                    }
                    PageLink {
                        locale,
                        page: PageKind::Demos,
                        class: if current_page == PageKind::Demos || current_page == PageKind::Bevy { " nav-link is-active".to_string() } else { " nav-link".to_string() },
                        label: nav_demos,
                    }
                    ExternalNavLink {
                        href: crate::site::routing::book_href(),
                        label: nav_docs,
                    }
                    ExternalNavLink {
                        href: "https://github.com/stayhydated/es-fluent".to_string(),
                        label: nav_source,
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

#[component]
fn LocaleSwitcher(locale: SiteLanguage, current_page: PageKind) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "locale-switcher", "failed: {error}" } },
    };
    let locale_label = i18n.localize_message(&SiteChromeMessage::LocaleLabel);
    let language_links = SiteLanguage::all()
        .map(|candidate| {
            let label = i18n.localize_message(&candidate);
            (candidate, label)
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "locale-switcher",
            span { class: "locale-label", "{locale_label}" }
            for (candidate, label) in language_links {
                PageLink {
                    locale: candidate,
                    page: current_page,
                    class: if candidate == locale { "locale-link is-active".to_string() } else { "locale-link".to_string() },
                    label,
                }
            }
        }
    }
}

#[component]
pub(crate) fn FooterPanel() -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { footer { class: "site-footer", "failed: {error}" } },
    };
    let label = i18n.localize_message(&SiteFooterMessage::Label);
    let body_prefix = i18n.localize_message(&SiteFooterMessage::BodyPrefix);
    let body_link_label = i18n.localize_message(&SiteFooterMessage::BodyLinkLabel);

    rsx! {
        footer { class: "site-footer",
            p { class: "footer-copy",
                span { class: "footer-label", "{label}" }
                span { class: "footer-text",
                    "{body_prefix} "
                    a {
                        class: "footer-link",
                        href: ES_FLUENT_MANAGER_DIOXUS_CRATES_URL,
                        target: "_blank",
                        rel: "noreferrer",
                        "{body_link_label}"
                    }
                    "."
                }
            }
        }
    }
}
