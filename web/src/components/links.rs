use crate::site::i18n::SiteLanguage;
use crate::site::routing::{PageKind, app_route, page_href};
use dioxus::prelude::*;

#[component]
pub(crate) fn PageLink(
    locale: SiteLanguage,
    page: PageKind,
    class: String,
    label: String,
) -> Element {
    if try_router().is_some() {
        rsx! {
            Link {
                class,
                to: app_route(locale, page),
                "{label}"
            }
        }
    } else {
        rsx! {
            a {
                class,
                href: page_href(locale, page),
                "{label}"
            }
        }
    }
}

#[component]
pub(crate) fn PageCardLink(
    locale: SiteLanguage,
    page: PageKind,
    label: String,
    title: String,
    body: String,
    action: String,
) -> Element {
    if try_router().is_some() {
        rsx! {
            Link {
                class: "demo-card",
                to: app_route(locale, page),
                div { class: "card-label", "{label}" }
                h2 { "{title}" }
                p { class: "card-copy", "{body}" }
                span { class: "card-link", "{action}" }
            }
        }
    } else {
        rsx! {
            a {
                class: "demo-card",
                href: page_href(locale, page),
                div { class: "card-label", "{label}" }
                h2 { "{title}" }
                p { class: "card-copy", "{body}" }
                span { class: "card-link", "{action}" }
            }
        }
    }
}
