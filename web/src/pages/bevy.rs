use crate::components::PageLink;
use crate::site::constants::BEVY_BOOTSTRAP;
use crate::site::i18n::{SiteLanguage, SiteMessage};
use crate::site::routing::PageKind;
use dioxus_core::Element;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent::ToFluentString as _;

#[component]
pub(crate) fn BevyPage(locale: SiteLanguage) -> Element {
    rsx! {
        div { class: "fullscreen-demo",
            PageLink {
                locale,
                page: PageKind::Demos,
                class: "back-pill".to_string(),
                label: SiteMessage::BackToDemos.to_fluent_string(),
            }
            div { class: "loader-stage",
                div { class: "loader-card", id: "bevy-loader", "data-state": "loading",
                    div { class: "loader-kicker", "{SiteMessage::BevyPageTitle.to_fluent_string()}" }
                    h1 { class: "loader-title", "{SiteMessage::BevyTitle.to_fluent_string()}" }
                    p { class: "loader-copy", "{SiteMessage::BevyLead.to_fluent_string()}" }
                    p { class: "status-line", "data-state": "loading", "{SiteMessage::BevyLoading.to_fluent_string()}" }
                    p { class: "status-line", "data-state": "error", "{SiteMessage::BevyError.to_fluent_string()}" }
                }
            }
            script {
                r#type: "module",
                dangerous_inner_html: "{BEVY_BOOTSTRAP}"
            }
        }
    }
}
