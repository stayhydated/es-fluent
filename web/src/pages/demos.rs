use dioxus::prelude::*;
use stayhydated_dioxus::{
    DemoGallery, DemoGalleryColumns, DemoGalleryItem, NavigationTarget,
    StayhydatedProjectPortalShell,
};

use crate::site::{
    constants::{PROJECT, VERSION},
    routing::{AppRoute, PageKind},
};

#[component]
pub(crate) fn DemosPage() -> Element {
    let demos = vec![
        DemoGalleryItem::route(
            crate::site::routing::app_route(PageKind::Dioxus),
            "Dioxus",
            "dioxus-demo-card-shader",
        ),
        DemoGalleryItem::route(
            crate::site::routing::app_route(PageKind::Bevy),
            "Bevy",
            "bevy-demo-card-shader",
        ),
        DemoGalleryItem::route(
            crate::site::routing::app_route(PageKind::Gpui),
            "GPUI",
            "gpui-demo-card-shader",
        ),
        DemoGalleryItem::route(
            crate::site::routing::app_route(PageKind::TypeScript),
            "TypeScript",
            "typescript-demo-card-shader",
        ),
    ];

    rsx! {
        StayhydatedProjectPortalShell {
            project: PROJECT,
            version: VERSION,
            home: NavigationTarget::Internal(crate::site::routing::app_route(PageKind::Home)),
            DemoGallery::<AppRoute> {
                items: demos,
                columns: DemoGalleryColumns::Two,
            }
        }
    }
}
