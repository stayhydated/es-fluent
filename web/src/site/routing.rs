use crate::pages::DevErrorPage;
use crate::site::i18n::{
    BevyPageMessage, GpuiPageMessage, HomeHeroMessage, PageMetadataMessage, SiteLanguage,
};
use dioxus::cli_config;
use dioxus::prelude::*;
use dioxus::router as dioxus_router;
use es_fluent_manager_dioxus::DioxusAssetI18nHandle;
use stayhydated_dioxus::{
    LocalizedRouteSegment, Project, ProjectNavItem, StayhydatedProjectPageMetadata,
    StayhydatedSiteLanguage,
};
use stayhydated_site::routing::{BaseHref, BasePath, Href, OutputDir, RoutePath};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageKind {
    Home,
    Demos,
    Bevy,
    Gpui,
}

impl PageKind {
    pub(crate) fn all() -> [Self; 4] {
        [Self::Home, Self::Demos, Self::Bevy, Self::Gpui]
    }

    pub(crate) fn route(self) -> &'static str {
        match self {
            Self::Home => "",
            Self::Demos => "demos",
            Self::Bevy => "bevy-example",
            Self::Gpui => "gpui-example",
        }
    }

    pub(crate) const fn project_nav_item(self) -> ProjectNavItem {
        match self {
            Self::Home => ProjectNavItem::Home,
            Self::Demos | Self::Bevy | Self::Gpui => ProjectNavItem::Demos,
        }
    }

    pub(crate) fn title(self, i18n: &DioxusAssetI18nHandle) -> String {
        match self {
            Self::Home => i18n.localize_message(&PageMetadataMessage::HomeTitle),
            Self::Demos => i18n.localize_message(&PageMetadataMessage::DemosTitle),
            Self::Bevy => i18n.localize_message(&PageMetadataMessage::BevyTitle),
            Self::Gpui => i18n.localize_message(&PageMetadataMessage::GpuiTitle),
        }
    }

    pub(crate) fn description(self, i18n: &DioxusAssetI18nHandle) -> String {
        match self {
            Self::Home => i18n.localize_message(&HomeHeroMessage::Body),
            Self::Demos => i18n.localize_message(&PageMetadataMessage::DemosDescription),
            Self::Bevy => i18n.localize_message(&BevyPageMessage::Lead),
            Self::Gpui => i18n.localize_message(&GpuiPageMessage::Lead),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SiteRoute {
    pub(crate) locale: SiteLanguage,
    pub(crate) page: PageKind,
}

impl SiteRoute {
    pub(crate) const fn new(locale: SiteLanguage, page: PageKind) -> Self {
        Self { locale, page }
    }

    pub(crate) fn output_dir(self) -> OutputDir {
        relative_path(self.locale, self.page).to_output_dir()
    }

    pub(crate) fn path(self) -> Href {
        stayhydated_site::routing::href(&BaseHref::root(), &relative_path(self.locale, self.page))
    }
}

pub(crate) fn all_routes() -> Vec<SiteRoute> {
    let mut routes = Vec::new();

    for locale in SiteLanguage::all_languages() {
        for page in PageKind::all() {
            routes.push(SiteRoute::new(locale, page));
        }
    }

    routes
}

pub(crate) fn app_base_href() -> BaseHref {
    let base_path = cli_config::base_path();
    let base_path = base_path.as_deref().map(BasePath::new);
    stayhydated_site::routing::base_href(base_path.as_ref())
}

pub(crate) fn page_href(locale: SiteLanguage, page: PageKind) -> Href {
    stayhydated_site::routing::href(&app_base_href(), &relative_path(locale, page))
}

pub(crate) fn book_href() -> Href {
    stayhydated_site::routing::href(&app_base_href(), &RoutePath::new("book"))
}

pub(crate) fn site_root_prefix(output_dir: &OutputDir) -> String {
    stayhydated_site::routing::site_root_prefix(output_dir)
}

pub(crate) type LocaleSegment = LocalizedRouteSegment<SiteLanguage>;

#[derive(Clone, Debug, Eq, PartialEq, Routable)]
#[rustfmt::skip]
pub(crate) enum AppRoute {
    #[route("/", HomeRoute)]
    Home {},
    #[route("/demos/", DemosRoute)]
    Demos {},
    #[route("/bevy-example/", BevyRoute)]
    Bevy {},
    #[route("/gpui-example/", GpuiRoute)]
    Gpui {},
    #[route("/:locale/", LocalizedHomeRoute)]
    LocalizedHome { locale: LocaleSegment },
    #[route("/:locale/demos/", LocalizedDemosRoute)]
    LocalizedDemos { locale: LocaleSegment },
    #[route("/:locale/bevy-example/", LocalizedBevyRoute)]
    LocalizedBevy { locale: LocaleSegment },
    #[route("/:locale/gpui-example/", LocalizedGpuiRoute)]
    LocalizedGpui { locale: LocaleSegment },
}

pub(crate) fn app_route(locale: SiteLanguage, page: PageKind) -> AppRoute {
    match (locale.route_slug(), page) {
        (None, PageKind::Home) => AppRoute::Home {},
        (None, PageKind::Demos) => AppRoute::Demos {},
        (None, PageKind::Bevy) => AppRoute::Bevy {},
        (None, PageKind::Gpui) => AppRoute::Gpui {},
        (Some(_), PageKind::Home) => AppRoute::LocalizedHome {
            locale: LocaleSegment::new(locale),
        },
        (Some(_), PageKind::Demos) => AppRoute::LocalizedDemos {
            locale: LocaleSegment::new(locale),
        },
        (Some(_), PageKind::Bevy) => AppRoute::LocalizedBevy {
            locale: LocaleSegment::new(locale),
        },
        (Some(_), PageKind::Gpui) => AppRoute::LocalizedGpui {
            locale: LocaleSegment::new(locale),
        },
    }
}

fn relative_path(locale: SiteLanguage, page: PageKind) -> RoutePath {
    let mut segments = Vec::new();

    if let Some(slug) = locale.route_slug() {
        segments.push(slug);
    }

    let page_segment = page.route();
    if !page_segment.is_empty() {
        segments.push(page_segment.to_string());
    }

    RoutePath::new(segments.join("/"))
}

fn route_element(route: SiteRoute) -> Element {
    let i18n = match es_fluent_manager_dioxus::use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => {
            return rsx!(DevErrorPage {
                route,
                message: format!("failed to access localization context: {error}"),
            });
        },
    };
    let route_language = route.locale.language_identifier();
    let i18n_result = if i18n.peek_requested_language() == route_language {
        Ok(i18n)
    } else {
        i18n.select_language(route_language)
            .map(|()| i18n)
            .map_err(|error| {
                format!(
                    "failed to select localized route '{}': {error}",
                    route.locale.html_lang()
                )
            })
    };

    match i18n_result {
        Ok(i18n) => {
            let _ = i18n.requested_language();
            let page_title = route.page.title(&i18n);
            let description = route.page.description(&i18n);

            rsx! {
                StayhydatedProjectPageMetadata {
                    project: Project::EsFluent,
                    page_title,
                    description,
                }
                {crate::pages::route_content(route)}
            }
        },
        Err(error) => rsx!(DevErrorPage {
            route,
            message: error,
        }),
    }
}

#[component]
fn HomeRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Home))
}

#[component]
fn DemosRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Demos))
}

#[component]
fn BevyRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Bevy))
}

#[component]
fn GpuiRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Gpui))
}

#[component]
fn LocalizedHomeRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Home))
}

#[component]
fn LocalizedDemosRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Demos))
}

#[component]
fn LocalizedBevyRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Bevy))
}

#[component]
fn LocalizedGpuiRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Gpui))
}
