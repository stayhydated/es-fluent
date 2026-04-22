use crate::site::i18n::SiteLanguage;
use crate::site::routing::PageKind;
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

#[cfg(not(feature = "web"))]
use crate::site::routing::page_href;
#[cfg(feature = "web")]
use crate::site::routing::page_route;
#[cfg(feature = "web")]
use dioxus::prelude::Link;

#[component]
pub(crate) fn PageLink(
    locale: SiteLanguage,
    page: PageKind,
    class: String,
    label: String,
) -> Element {
    #[cfg(feature = "web")]
    {
        let route = page_route(locale, page);
        rsx! {
            Link {
                class,
                to: route,
                "{label}"
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
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
    #[cfg(feature = "web")]
    {
        let route = page_route(locale, page);
        rsx! {
            Link {
                class: "demo-card",
                to: route,
                div { class: "card-label", "{label}" }
                h2 { "{title}" }
                p { class: "card-copy", "{body}" }
                span { class: "card-link", "{action}" }
            }
        }
    }

    #[cfg(not(feature = "web"))]
    {
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
