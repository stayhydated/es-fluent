use crate::components::PageHeader;
use crate::site::routing::SiteRoute;
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;

#[component]
pub(crate) fn DevErrorPage(route: SiteRoute, message: String) -> Element {
    rsx! {
        div { class: "page-shell",
            PageHeader { locale: route.locale, current_page: route.page }
            main { class: "stack",
                section { class: "section-panel",
                    h1 { class: "section-title", "Localized route failed" }
                    p { class: "section-lead", "The page could not initialize its Dioxus i18n context." }
                    pre { code { "{message}" } }
                }
            }
        }
    }
}
