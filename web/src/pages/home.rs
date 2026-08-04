use dioxus::prelude::*;
use stayhydated_dioxus::{NavigationTarget, StayhydatedProjectSitePortal};

use crate::site::{constants::site, routing::PageKind};

#[component]
pub(crate) fn HomePage() -> Element {
    rsx! {
        StayhydatedProjectSitePortal::<crate::site::routing::AppRoute> {
            site: site(),
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            demos: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Demos)),
        }
    }
}
