#[cfg(feature = "web")]
use crate::pages::{DevErrorPage, route_content};
use crate::site::i18n::{SiteLanguage, SiteMessage};
#[cfg(feature = "web")]
use dioxus::prelude::{Routable, VNode};
#[cfg(feature = "web")]
use dioxus::router as dioxus_router;
#[cfg(feature = "web")]
use dioxus_core::Element;
#[cfg(feature = "web")]
use dioxus_core::use_hook;
#[cfg(feature = "web")]
use dioxus_core_macro::{Props, component, rsx};
#[cfg(feature = "web")]
use es_fluent_manager_dioxus::ManagedI18n;
#[cfg(feature = "web")]
use es_fluent_manager_dioxus::{GlobalLocalizerMode, web::use_provide_i18n_with_mode};
#[cfg(feature = "web")]
use std::fmt::{self, Display};
#[cfg(feature = "web")]
use std::str::FromStr;

#[cfg(test)]
const SITE_BASE_PATH_SEGMENT: &str = "es-fluent";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PageKind {
    Home,
    Demos,
    Bevy,
}

impl PageKind {
    pub(crate) fn all() -> [Self; 3] {
        [Self::Home, Self::Demos, Self::Bevy]
    }

    pub(crate) fn route(self) -> &'static str {
        match self {
            Self::Home => "",
            Self::Demos => "demos",
            Self::Bevy => "bevy-example",
        }
    }

    pub(crate) fn title_message(self) -> SiteMessage {
        match self {
            Self::Home => SiteMessage::HomePageTitle,
            Self::Demos => SiteMessage::DemosPageTitle,
            Self::Bevy => SiteMessage::BevyPageTitle,
        }
    }

    pub(crate) fn description_message(self) -> SiteMessage {
        match self {
            Self::Home => SiteMessage::HeroBody,
            Self::Demos => SiteMessage::DemosLead,
            Self::Bevy => SiteMessage::BevyLead,
        }
    }

    pub(crate) fn is_fullscreen(self) -> bool {
        matches!(self, Self::Bevy)
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

    pub(crate) fn output_dir(self) -> String {
        relative_path(self.locale, self.page)
    }
}

pub(crate) fn app_base_href() -> &'static str {
    "/es-fluent/"
}

pub(crate) fn page_href(locale: SiteLanguage, page: PageKind) -> String {
    let relative = relative_path(locale, page);
    if relative.is_empty() {
        app_base_href().to_string()
    } else {
        format!("{}{relative}/", app_base_href())
    }
}

pub(crate) fn book_href() -> String {
    format!("{}book/", app_base_href())
}

pub(crate) fn site_root_prefix(output_dir: &str) -> String {
    if output_dir.is_empty() {
        return "./".to_string();
    }

    let depth = output_dir.split('/').count();
    "../".repeat(depth)
}

#[cfg(feature = "web")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocaleSegment(SiteLanguage);

#[cfg(feature = "web")]
impl LocaleSegment {
    fn language(&self) -> SiteLanguage {
        self.0
    }
}

#[cfg(feature = "web")]
impl Display for LocaleSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.route_slug() {
            Some(slug) => f.write_str(&slug),
            None => Err(fmt::Error),
        }
    }
}

#[cfg(feature = "web")]
impl FromStr for LocaleSegment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SiteLanguage::from_route_slug(s)
            .map(Self)
            .ok_or_else(|| format!("unsupported locale route segment: {s}"))
    }
}

#[cfg(feature = "web")]
#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub(crate) enum AppRoute {
    #[route("/", HomeRoute)]
    Home {},
    #[route("/demos/", DemosRoute)]
    Demos {},
    #[route("/bevy-example/", BevyRoute)]
    Bevy {},
    #[route("/:locale/", LocalizedHomeRoute)]
    LocalizedHome { locale: LocaleSegment },
    #[route("/:locale/demos/", LocalizedDemosRoute)]
    LocalizedDemos { locale: LocaleSegment },
    #[route("/:locale/bevy-example/", LocalizedBevyRoute)]
    LocalizedBevy { locale: LocaleSegment },
}

#[cfg(feature = "web")]
impl AppRoute {
    fn from_site_route(route: SiteRoute) -> Self {
        match (route.locale.is_default(), route.page) {
            (true, PageKind::Home) => Self::Home {},
            (true, PageKind::Demos) => Self::Demos {},
            (true, PageKind::Bevy) => Self::Bevy {},
            (false, PageKind::Home) => Self::LocalizedHome {
                locale: LocaleSegment(route.locale),
            },
            (false, PageKind::Demos) => Self::LocalizedDemos {
                locale: LocaleSegment(route.locale),
            },
            (false, PageKind::Bevy) => Self::LocalizedBevy {
                locale: LocaleSegment(route.locale),
            },
        }
    }
}

#[cfg(feature = "web")]
pub(crate) fn page_route(locale: SiteLanguage, page: PageKind) -> AppRoute {
    AppRoute::from_site_route(SiteRoute::new(locale, page))
}

#[cfg(test)]
pub(crate) fn site_route_from_path(path: &str) -> SiteRoute {
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .filter(|segment| *segment != SITE_BASE_PATH_SEGMENT)
        .collect::<Vec<_>>();

    let (locale, page_segments) = match segments.split_first() {
        Some((first, rest)) => match SiteLanguage::from_route_slug(first) {
            Some(locale) => (locale, rest),
            None => (SiteLanguage::default(), segments.as_slice()),
        },
        None => (SiteLanguage::default(), &[][..]),
    };

    SiteRoute::new(locale, page_from_segments(page_segments))
}

#[cfg(test)]
fn page_from_segments(segments: &[&str]) -> PageKind {
    match segments {
        [] => PageKind::Home,
        ["demos"] => PageKind::Demos,
        ["bevy-example"] => PageKind::Bevy,
        _ => PageKind::Home,
    }
}

fn relative_path(locale: SiteLanguage, page: PageKind) -> String {
    let mut segments = Vec::new();

    if let Some(slug) = locale.route_slug() {
        segments.push(slug);
    }

    let page_segment = page.route();
    if !page_segment.is_empty() {
        segments.push(page_segment.to_string());
    }

    segments.join("/")
}

#[cfg(feature = "web")]
fn route_element(route: SiteRoute) -> Element {
    let init_result = use_hook(|| {
        ManagedI18n::try_new_with_discovered_modules(route.locale.lang()).map_err(|error| {
            format!(
                "failed to initialize localized route '{}': {error}",
                route.locale.html_lang()
            )
        })
    });

    match init_result.as_ref() {
        Ok(managed) => {
            let _i18n =
                use_provide_i18n_with_mode(managed.clone(), GlobalLocalizerMode::ReplaceExisting);
            route_content(route)
        },
        Err(error) => rsx!(DevErrorPage {
            route,
            message: error.clone(),
        }),
    }
}

#[cfg(feature = "web")]
#[component]
fn HomeRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Home))
}

#[cfg(feature = "web")]
#[component]
fn DemosRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Demos))
}

#[cfg(feature = "web")]
#[component]
fn BevyRoute() -> Element {
    route_element(SiteRoute::new(SiteLanguage::default(), PageKind::Bevy))
}

#[cfg(feature = "web")]
#[component]
fn LocalizedHomeRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Home))
}

#[cfg(feature = "web")]
#[component]
fn LocalizedDemosRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Demos))
}

#[cfg(feature = "web")]
#[component]
fn LocalizedBevyRoute(locale: LocaleSegment) -> Element {
    route_element(SiteRoute::new(locale.language(), PageKind::Bevy))
}
