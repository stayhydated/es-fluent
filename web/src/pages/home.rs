use dioxus::prelude::*;
use stayhydated_dioxus::{Href, NavigationTarget, StayhydatedProjectPortal};

use crate::site::{
    constants::{PROJECT, VERSION},
    routing::PageKind,
};

#[component]
pub(crate) fn HomePage() -> Element {
    rsx! {
        StayhydatedProjectPortal::<crate::site::routing::AppRoute> {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            book: Href::new(crate::site::routing::book_href().into_string()),
            demos: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Demos)),
        }
    }
}
