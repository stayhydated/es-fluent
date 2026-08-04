use crate::pages;
use dioxus::prelude::*;
use stayhydated_dioxus::StayhydatedProjectPageMetadata;
use stayhydated_site::routing::{OutputDir, RoutePath};

use crate::site::constants::PROJECT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageKind {
    Home,
    Demos,
    Dioxus,
    Bevy,
    Gpui,
}

impl PageKind {
    fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Demos => "Demos",
            Self::Dioxus => "Osmose Demo",
            Self::Bevy => "Bevy Example",
            Self::Gpui => "GPUI Example",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Home => {
                "Define messages in Rust. Generate Fluent files. Use them in embedded, Bevy, and Dioxus apps."
            },
            Self::Demos => "Browser demos for es-fluent runtime integrations.",
            Self::Dioxus => "A hero demo for the Expressive E Osmose 49/61-key MPE synthesizer.",
            Self::Bevy => "A Bevy wasm demo inside the book site.",
            Self::Gpui => "A GPUI wasm demo inside the book site.",
        }
    }
}

pub(crate) fn site_root_prefix(output_dir: &OutputDir) -> String {
    stayhydated_site::routing::site_root_prefix(output_dir)
}

#[derive(Clone, Debug, Eq, PartialEq, Routable)]
#[rustfmt::skip]
pub(crate) enum AppRoute {
    #[route("/", HomeRoute)]
    Home {},
    #[route("/demos/", DemosRoute)]
    Demos {},
    #[route("/dioxus-example/", DioxusRoute)]
    Dioxus {},
    #[route("/bevy-example/", BevyRoute)]
    Bevy {},
    #[route("/gpui-example/", GpuiRoute)]
    Gpui {},
}

pub(crate) fn app_route(page: PageKind) -> AppRoute {
    match page {
        PageKind::Home => AppRoute::Home {},
        PageKind::Demos => AppRoute::Demos {},
        PageKind::Dioxus => AppRoute::Dioxus {},
        PageKind::Bevy => AppRoute::Bevy {},
        PageKind::Gpui => AppRoute::Gpui {},
    }
}

pub(crate) fn output_dir(page: PageKind) -> OutputDir {
    RoutePath::new(app_route(page).to_string()).to_output_dir()
}

fn route_element(page: PageKind) -> Element {
    rsx! {
        StayhydatedProjectPageMetadata {
            project: PROJECT,
            page_title: page.title(),
            description: page.description(),
        }
        {pages::route_content(page)}
    }
}

#[component]
fn HomeRoute() -> Element {
    route_element(PageKind::Home)
}

#[component]
fn DemosRoute() -> Element {
    route_element(PageKind::Demos)
}

#[component]
fn DioxusRoute() -> Element {
    route_element(PageKind::Dioxus)
}

#[component]
fn BevyRoute() -> Element {
    route_element(PageKind::Bevy)
}

#[component]
fn GpuiRoute() -> Element {
    route_element(PageKind::Gpui)
}
