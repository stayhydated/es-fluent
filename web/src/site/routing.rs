use crate::pages::{DevErrorPage, route_content};
use crate::site::i18n::{SiteLanguage, SiteMessage};
use dioxus::cli_config;
use dioxus::prelude::{Meta, Routable, Title, VNode};
use dioxus::router as dioxus_router;
use dioxus_core::Element;
use dioxus_core::use_hook;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent::ToFluentString as _;
use es_fluent_manager_dioxus::ManagedI18n;
use es_fluent_manager_dioxus::{GlobalLocalizerMode, use_provide_i18n_with_mode};
use std::fmt::{self, Display};
use std::str::FromStr;

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
            Self::Demos => SiteMessage::DemoBevyBody,
            Self::Bevy => SiteMessage::BevyLead,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
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

pub(crate) fn app_base_href() -> String {
    match cli_config::base_path() {
        Some(base_path) => {
            let base_path = base_path.trim_matches('/');
            if base_path.is_empty() {
                "/".to_string()
            } else {
                format!("/{base_path}/")
            }
        },
        None => "/".to_string(),
    }
}

pub(crate) fn page_href(locale: SiteLanguage, page: PageKind) -> String {
    let relative = relative_path(locale, page);
    if relative.is_empty() {
        app_base_href()
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocaleSegment(SiteLanguage);

impl LocaleSegment {
    fn language(&self) -> SiteLanguage {
        self.0
    }
}

impl Display for LocaleSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.route_slug() {
            Some(slug) => f.write_str(&slug),
            None => Err(fmt::Error),
        }
    }
}

impl FromStr for LocaleSegment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SiteLanguage::from_route_slug(s)
            .map(Self)
            .ok_or_else(|| format!("unsupported locale route segment: {s}"))
    }
}

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

#[cfg(test)]
pub(crate) fn site_route_from_path(path: &str) -> SiteRoute {
    site_route_from_path_with_base_path(path, None)
}

#[cfg(test)]
pub(crate) fn site_route_from_path_with_base_path(
    path: &str,
    base_path: Option<&str>,
) -> SiteRoute {
    let segments = normalized_path_segments(path, base_path);

    let (locale, page_segments) = match segments.as_slice().split_first() {
        Some((first, rest)) => match SiteLanguage::from_route_slug(first) {
            Some(locale) => (locale, rest),
            None => (SiteLanguage::default(), segments.as_slice()),
        },
        None => (SiteLanguage::default(), &[][..]),
    };

    SiteRoute::new(locale, page_from_segments(page_segments))
}

#[cfg(test)]
fn normalized_path_segments<'a>(path: &'a str, base_path: Option<&str>) -> Vec<&'a str> {
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    let base_path_segments = base_path
        .into_iter()
        .flat_map(|base_path| base_path.trim_matches('/').split('/'))
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if base_path_segments.is_empty()
        || !segments
            .as_slice()
            .starts_with(base_path_segments.as_slice())
    {
        segments
    } else {
        segments[base_path_segments.len()..].to_vec()
    }
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
            let title = format!(
                "{} | {}",
                SiteMessage::SiteName.to_fluent_string(),
                route.page.title_message().to_fluent_string()
            );
            let description = route.page.description_message().to_fluent_string();

            rsx! {
                Title { "{title}" }
                Meta {
                    name: "description",
                    content: description,
                }
                {route_content(route)}
            }
        },
        Err(error) => rsx!(DevErrorPage {
            route,
            message: error.clone(),
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
