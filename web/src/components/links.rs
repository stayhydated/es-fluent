use crate::site::i18n::SiteLanguage;
use crate::site::routing::{PageKind, page_href};
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

#[component]
pub(crate) fn PageLink(
    locale: SiteLanguage,
    page: PageKind,
    class: String,
    label: String,
) -> Element {
    rsx! {
        a {
            class,
            href: page_href(locale, page),
            "{label}"
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
