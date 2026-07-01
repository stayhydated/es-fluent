use crate::components::{FooterPanel, PageHeader};
use crate::site::routing::PageKind;
use dioxus::prelude::*;
use stayhydated_dioxus::{DemoCard, DemoCardGrid, ProjectPageShell, page_entry_reveal_style};

#[component]
pub(crate) fn DemosPage() -> Element {
    let demos_style = page_entry_reveal_style();

    rsx! {
        ProjectPageShell {
            header: rsx!(PageHeader { current_page: PageKind::Demos }),
            footer: Some(rsx!(FooterPanel {})),
            DemoCardGrid::<crate::site::routing::AppRoute> {
                cards: vec![
                    DemoCard::route(
                        crate::site::routing::app_route(PageKind::Dioxus),
                        "osmose",
                        "Osmose synthesizer",
                        "A hero demo for the Expressive E Osmose 49/61-key MPE synthesizer.",
                        "Launch demo",
                    ),
                    DemoCard::route(
                        crate::site::routing::app_route(PageKind::Bevy),
                        "wasm demo",
                        "Bevy example",
                        "Bevy wasm in the browser.",
                        "Launch demo",
                    ),
                    DemoCard::route(
                        crate::site::routing::app_route(PageKind::Gpui),
                        "wasm demo",
                        "GPUI example",
                        "GPUI wasm in WebGPU browsers.",
                        "Launch demo",
                    ),
                ],
                extra_class: "motion-reveal",
                style: demos_style,
                body_class: "card-copy",
            }
        }
    }
}
