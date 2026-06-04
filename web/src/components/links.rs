use crate::site::i18n::SiteLanguage;
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{CssClass, DisplayText, LinkTarget, RouteCardLink, RouteLink};

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
            class: CssClass::new(class),
            label: DisplayText::new(label),
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
            label: DisplayText::new(label),
            title: DisplayText::new(title),
            body: DisplayText::new(body),
            body_class: CssClass::new("card-copy"),
            action: DisplayText::new(action),
        }
    }
}
