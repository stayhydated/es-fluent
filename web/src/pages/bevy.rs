use crate::components::PageLink;
use crate::site::i18n::{BevyPageMessage, SiteLanguage};
use crate::site::routing::{PageKind, SiteRoute, site_root_prefix};
use dioxus::prelude::*;
use es_fluent::ToFluentString as _;
use es_fluent_manager_dioxus_derive::i18n_subscription;

#[i18n_subscription]
#[component]
pub(crate) fn BevyPage(locale: SiteLanguage) -> Element {
    let page_output_dir = SiteRoute::new(locale, PageKind::Bevy).output_dir();
    let demo_src = format!("{}bevy-demo/", site_root_prefix(&page_output_dir));

    rsx! {
        div { class: "fullscreen-demo",
            PageLink {
                locale,
                page: PageKind::Demos,
                class: "back-pill".to_string(),
                label: BevyPageMessage::BackToDemos.to_fluent_string(),
            }
            iframe {
                class: "fullscreen-demo-frame",
                src: demo_src,
                title: BevyPageMessage::Title.to_fluent_string(),
                allowfullscreen: true,
            }
        }
    }
}
