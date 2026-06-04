use crate::site::i18n::SiteLanguage;
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{LinkTarget, RouteCardLink, RouteLink};

#[component]
pub(crate) fn PageLink(
    locale: SiteLanguage,
    page: PageKind,
    class: String,
    label: String,
) -> Element {
    rsx! {
        RouteLink {
            target: LinkTarget::route(crate::site::routing::app_route(locale, page)),
            class,
            label,
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
    rsx! {
        RouteCardLink {
            target: LinkTarget::route(crate::site::routing::app_route(locale, page)),
            label,
            title,
            body,
            body_class: "card-copy".to_string(),
            action,
        }
    }
}
