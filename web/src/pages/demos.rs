use dioxus::prelude::*;
use stayhydated_dioxus::{
    NavigationTarget, StayhydatedProjectPortalShell, page_entry_reveal_style,
};

use crate::site::{
    constants::{PROJECT, VERSION},
    routing::{AppRoute, PageKind},
};

#[component]
fn DemoCardLink(route: AppRoute, title: &'static str) -> Element {
    let aria_label = format!("Open {title} demo");

    if try_router().is_some() {
        rsx! {
            Link {
                class: "demo-card",
                to: route,
                aria_label,
                h2 { class: "demo-card-title", "{title}" }
            }
        }
    } else {
        rsx! {
            a {
                class: "demo-card",
                href: route.to_string(),
                aria_label,
                h2 { class: "demo-card-title", "{title}" }
            }
        }
    }
}

#[component]
pub(crate) fn DemosPage() -> Element {
    let demos_style = page_entry_reveal_style().into_string();

    rsx! {
        StayhydatedProjectPortalShell {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            div { class: "demo-page demo-gallery",
                section {
                    class: "grid columns-3 demo-example-cards motion-reveal",
                    style: demos_style,
                    DemoCardLink {
                        route: crate::site::routing::app_route(PageKind::Dioxus),
                        title: "Dioxus",
                    }
                    DemoCardLink {
                        route: crate::site::routing::app_route(PageKind::Bevy),
                        title: "Bevy",
                    }
                    DemoCardLink {
                        route: crate::site::routing::app_route(PageKind::Gpui),
                        title: "GPUI",
                    }
                }
            }
        }
    }
}
