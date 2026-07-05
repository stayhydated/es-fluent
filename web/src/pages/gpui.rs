use crate::site::routing::{PageKind, SiteRoute};
use dioxus::prelude::*;
use stayhydated_dioxus::{FullscreenDemoPage, LinkTarget};

#[component]
pub(crate) fn GpuiPage() -> Element {
    let page_output_dir = SiteRoute::new(PageKind::Gpui).output_dir();
    let demo_src = format!(
        "{}gpui-demo/",
        crate::site::routing::site_root_prefix(&page_output_dir)
    );

    rsx! {
        FullscreenDemoPage::<crate::site::routing::AppRoute> {
            back_target: LinkTarget::route(crate::site::routing::app_route(PageKind::Demos)),
            back_label: "Back to demos",
            src: demo_src,
            title: "GPUI example",
        }
    }
}
