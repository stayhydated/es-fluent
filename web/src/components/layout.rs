use crate::components::PageLink;
use crate::site::constants::{CRATES_URL, README_URL};
use crate::site::i18n::{SiteLanguage, SiteMessage};
use crate::site::routing::{PageKind, book_href, page_href};
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent::ToFluentString as _;

#[component]
pub(crate) fn PageHeader(locale: SiteLanguage, current_page: PageKind) -> Element {
    rsx! {
        header { class: "page-header",
            a { class: "brand", href: page_href(locale, PageKind::Home),
                span { class: "brand-mark", "EF" }
                span { class: "brand-copy",
                    span { class: "brand-kicker", "{SiteMessage::PageKicker.to_fluent_string()}" }
                    span { class: "brand-title", "{SiteMessage::SiteName.to_fluent_string()}" }
                }
            }
            div { class: "header-cluster",
                nav { class: "nav-pill",
                    PageLink {
                        locale,
                        page: PageKind::Home,
                        class: if current_page == PageKind::Home { " nav-link is-active".to_string() } else { " nav-link".to_string() },
                        label: SiteMessage::NavHome.to_fluent_string(),
                    }
                    PageLink {
                        locale,
                        page: PageKind::Demos,
                        class: if current_page == PageKind::Demos || current_page == PageKind::Bevy { " nav-link is-active".to_string() } else { " nav-link".to_string() },
                        label: SiteMessage::NavDemos.to_fluent_string(),
                    }
                    ExternalNavLink {
                        href: book_href(),
                        label: SiteMessage::NavDocs.to_fluent_string(),
                    }
                    ExternalNavLink {
                        href: "https://github.com/stayhydated/es-fluent".to_string(),
                        label: SiteMessage::NavSource.to_fluent_string(),
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
    rsx! {
        div { class: "locale-switcher",
            span { class: "locale-label", "{SiteMessage::LocaleLabel.to_fluent_string()}" }
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

#[component]
pub(crate) fn FooterPanel() -> Element {
    rsx! {
        footer { class: "footer-panel",
            div {
                div { class: "footer-label", "{SiteMessage::FooterLabel.to_fluent_string()}" }
                p { class: "footer-copy", "{SiteMessage::FooterBody.to_fluent_string()}" }
            }
            ul { class: "footer-links",
                li { a { class: "text-link", href: book_href(), "{SiteMessage::LinksBook.to_fluent_string()}" } }
                li { a { class: "text-link", href: README_URL, target: "_blank", rel: "noreferrer", "{SiteMessage::LinksReadme.to_fluent_string()}" } }
                li { a { class: "text-link", href: CRATES_URL, target: "_blank", rel: "noreferrer", "{SiteMessage::LinksCrates.to_fluent_string()}" } }
            }
        }
    }
}
