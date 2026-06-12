use crate::site::i18n::{GpuiPageMessage, SiteLanguage};
use crate::site::routing::{PageKind, SiteRoute};
use dioxus::prelude::*;
use stayhydated_dioxus::{BackLink, LinkTarget};

#[component]
pub(crate) fn GpuiPage(locale: SiteLanguage) -> Element {
    let page_output_dir = SiteRoute::new(locale, PageKind::Gpui).output_dir();
    let demo_src = format!(
        "{}gpui-demo/",
        crate::site::routing::site_root_prefix(&page_output_dir)
    );
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => return rsx! { div { class: "fullscreen-demo", "failed: {error}" } },
    };
    let back_to_demos = i18n.localize_message(&GpuiPageMessage::BackToDemos);
    let title = i18n.localize_message(&GpuiPageMessage::Title);

    rsx! {
        div { class: "fullscreen-demo",
            BackLink::<crate::site::routing::AppRoute> {
                target: LinkTarget::route(crate::site::routing::app_route(locale, PageKind::Demos)),
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
