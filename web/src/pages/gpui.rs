use crate::site::{
    constants::{PROJECT, VERSION},
    routing::{PageKind, SiteRoute},
};
use dioxus::prelude::*;
use stayhydated_dioxus::{FullscreenDemoFrame, NavigationTarget, StayhydatedProjectPortalShell};

#[component]
pub(crate) fn GpuiPage() -> Element {
    let page_output_dir = SiteRoute::new(PageKind::Gpui).output_dir();
    let demo_src = format!(
        "{}gpui-demo/",
        crate::site::routing::site_root_prefix(&page_output_dir)
    );

    rsx! {
        StayhydatedProjectPortalShell {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            FullscreenDemoFrame {
                src: demo_src,
                title: "GPUI example",
            }
        }
    }
}
