use crate::site::i18n::{BevyPageMessage, SiteLanguage};
use crate::site::routing::{PageKind, SiteRoute};
use dioxus::prelude::*;
use stayhydated_dioxus::{FullscreenDemoPage, LinkTarget};

#[component]
pub(crate) fn BevyPage(locale: SiteLanguage) -> Element {
    let page_output_dir = SiteRoute::new(locale, PageKind::Bevy).output_dir();
    let demo_src = format!(
        "{}bevy-demo/",
        crate::site::routing::site_root_prefix(&page_output_dir)
    );
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "fullscreen-demo", "failed: {error}" } },
    };
    let back_to_demos = i18n.localize_message(&BevyPageMessage::BackToDemos);
    let title = i18n.localize_message(&BevyPageMessage::Title);

    rsx! {
        FullscreenDemoPage::<crate::site::routing::AppRoute> {
            back_target: LinkTarget::route(crate::site::routing::app_route(locale, PageKind::Demos)),
            back_label: back_to_demos,
            src: demo_src,
            title,
        }
    }
}
