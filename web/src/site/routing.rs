use crate::pages;
use dioxus::cli_config;
use dioxus::prelude::*;
use stayhydated_dioxus::{Project, ProjectNavItem, StayhydatedProjectPageMetadata};
use stayhydated_site::routing::{BaseHref, BasePath, Href, OutputDir, RoutePath};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageKind {
    Home,
    Demos,
    Dioxus,
    Bevy,
    Gpui,
}

impl PageKind {
    pub(crate) fn all() -> [Self; 5] {
        [
            Self::Home,
            Self::Demos,
            Self::Dioxus,
            Self::Bevy,
            Self::Gpui,
        ]
    }

    pub(crate) fn route(self) -> &'static str {
        match self {
            Self::Home => "",
            Self::Demos => "demos",
            Self::Dioxus => "dioxus-example",
            Self::Bevy => "bevy-example",
            Self::Gpui => "gpui-example",
        }
    }

    pub(crate) const fn project_nav_item(self) -> ProjectNavItem {
        match self {
            Self::Home => ProjectNavItem::Home,
            Self::Demos | Self::Dioxus | Self::Bevy | Self::Gpui => ProjectNavItem::Demos,
        }
    }

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SiteRoute {
    pub(crate) page: PageKind,
}

impl SiteRoute {
    pub(crate) const fn new(page: PageKind) -> Self {
        Self { page }
    }

    pub(crate) fn output_dir(self) -> OutputDir {
        relative_path(self.page).to_output_dir()
    }

    pub(crate) fn path(self) -> Href {
        stayhydated_site::routing::href(&BaseHref::root(), &relative_path(self.page))
    }
}

pub(crate) fn all_routes() -> Vec<SiteRoute> {
    PageKind::all().into_iter().map(SiteRoute::new).collect()
}

pub(crate) fn app_base_href() -> BaseHref {
    let base_path = cli_config::base_path();
    let base_path = base_path.as_deref().map(BasePath::new);
    stayhydated_site::routing::base_href(base_path.as_ref())
}

pub(crate) fn page_href(page: PageKind) -> Href {
    stayhydated_site::routing::href(&app_base_href(), &relative_path(page))
}

pub(crate) fn book_href() -> Href {
    stayhydated_site::routing::href(&app_base_href(), &RoutePath::new("book"))
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

fn relative_path(page: PageKind) -> RoutePath {
    RoutePath::new(page.route())
}

fn route_element(route: SiteRoute) -> Element {
    rsx! {
        StayhydatedProjectPageMetadata {
            project: Project::EsFluent,
            page_title: route.page.title(),
            description: route.page.description(),
        }
        {pages::route_content(route)}
    }
}

#[component]
fn HomeRoute() -> Element {
    route_element(SiteRoute::new(PageKind::Home))
}

#[component]
fn DemosRoute() -> Element {
    route_element(SiteRoute::new(PageKind::Demos))
}

#[component]
fn DioxusRoute() -> Element {
    route_element(SiteRoute::new(PageKind::Dioxus))
}

#[component]
fn BevyRoute() -> Element {
    route_element(SiteRoute::new(PageKind::Bevy))
}

#[component]
fn GpuiRoute() -> Element {
    route_element(SiteRoute::new(PageKind::Gpui))
}
