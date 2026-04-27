use crate::components::PageLink;
use crate::site::i18n::{BevyPageMessage, SiteLanguage};
use crate::site::routing::{PageKind, SiteRoute, site_root_prefix};
use dioxus::prelude::*;
use es_fluent_manager_dioxus::use_i18n;

#[component]
pub(crate) fn BevyPage(locale: SiteLanguage) -> Element {
    let page_output_dir = SiteRoute::new(locale, PageKind::Bevy).output_dir();
    let demo_src = format!("{}bevy-demo/", site_root_prefix(&page_output_dir));
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "fullscreen-demo", "failed: {error}" } },
    };
    let back_to_demos = i18n.localize_message(&BevyPageMessage::BackToDemos);
    let title = i18n.localize_message(&BevyPageMessage::Title);

    rsx! {
        div { class: "fullscreen-demo",
            PageLink {
                locale,
                page: PageKind::Demos,
                class: "back-pill".to_string(),
                label: back_to_demos,
            }
            iframe {
                class: "fullscreen-demo-frame",
                src: demo_src,
                title,
                allowfullscreen: true,
            }
        }
    }
}
