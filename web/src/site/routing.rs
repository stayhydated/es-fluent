use crate::pages::{DevErrorPage, route_content};
use crate::site::i18n::{
    BevyPageMessage, DemosPageMessage, HomeHeroMessage, PageMetadataMessage, SiteChromeMessage,
    SiteLanguage,
};
use dioxus::cli_config;
use dioxus::prelude::*;
use dioxus::router as dioxus_router;
use es_fluent_lang::LanguageIdentifier;
use es_fluent_manager_dioxus::{DioxusI18n, use_i18n};
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::fs;
use std::path::Path;
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

    pub(crate) fn title(self, i18n: &DioxusI18n) -> String {
        match self {
            Self::Home => i18n.localize_message(&PageMetadataMessage::Home),
            Self::Demos => i18n.localize_message(&PageMetadataMessage::Demos),
            Self::Bevy => i18n.localize_message(&PageMetadataMessage::Bevy),
        }
    }

    pub(crate) fn description(self, i18n: &DioxusI18n) -> String {
        match self {
            Self::Home => i18n.localize_message(&HomeHeroMessage::Body),
            Self::Demos => i18n.localize_message(&DemosPageMessage::Body),
            Self::Bevy => i18n.localize_message(&BevyPageMessage::Lead),
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

    pub(crate) fn output_dir(self) -> String {
        relative_path(self.locale, self.page)
    }

    pub(crate) fn path(self) -> String {
        let relative = self.output_dir();

        if relative.is_empty() {
            "/".to_string()
        } else {
            format!("/{relative}/")
        }
    }
}

pub(crate) fn all_routes() -> Vec<SiteRoute> {
    let mut routes = Vec::new();

    for locale in SiteLanguage::all() {
        for page in PageKind::all() {
            routes.push(SiteRoute::new(locale, page));
        }
    }

    routes
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

pub(crate) fn app_route(locale: SiteLanguage, page: PageKind) -> AppRoute {
    match (locale.route_slug(), page) {
        (None, PageKind::Home) => AppRoute::Home {},
        (None, PageKind::Demos) => AppRoute::Demos {},
        (None, PageKind::Bevy) => AppRoute::Bevy {},
        (Some(_), PageKind::Home) => AppRoute::LocalizedHome {
            locale: LocaleSegment(locale),
        },
        (Some(_), PageKind::Demos) => AppRoute::LocalizedDemos {
            locale: LocaleSegment(locale),
        },
        (Some(_), PageKind::Bevy) => AppRoute::LocalizedBevy {
            locale: LocaleSegment(locale),
        },
    }
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

const GENERATED_ROUTE_CACHE_MARKER: &str = ".es-fluent-generated-route-cache";

pub(crate) fn mark_generated_route_cache(public_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(public_dir)?;
    fs::write(
        public_dir.join(GENERATED_ROUTE_CACHE_MARKER),
        "Generated route cache owned by es-fluent web server.\n",
    )
}

pub(crate) fn cleanup_generated_route_cache(public_dir: &Path) -> std::io::Result<()> {
    if !public_dir.exists() {
        return Ok(());
    }

    if !public_dir.join(GENERATED_ROUTE_CACHE_MARKER).is_file() {
        return Ok(());
    }

    remove_file_if_exists(&public_dir.join("index.html"))?;
    remove_file_if_exists(&public_dir.join("404.html"))?;

    let generated_top_level_dirs = all_routes()
        .into_iter()
        .filter_map(|route| {
            route
                .output_dir()
                .split('/')
                .next()
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
        })
        .collect::<HashSet<_>>();

    for dir in &generated_top_level_dirs {
        remove_dir_if_exists(&public_dir.join(dir))?;
    }

    for entry in fs::read_dir(public_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if is_locale_route_dir(name) && contains_generated_route_cache(&entry.path()) {
            fs::remove_dir_all(entry.path())?;
        }
    }

    Ok(())
}

fn remove_file_if_exists(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    }

    Ok(())
}

fn is_locale_route_dir(name: &str) -> bool {
    name.parse::<LanguageIdentifier>().is_ok()
}

fn contains_generated_route_cache(dir: &Path) -> bool {
    dir.join("index.html").is_file()
        || PageKind::all()
            .into_iter()
            .map(PageKind::route)
            .filter(|route| !route.is_empty())
            .any(|route| dir.join(route).is_dir())
}

fn route_element(route: SiteRoute) -> Element {
    let i18n = match use_i18n() {
        Ok(i18n) => i18n,
        Err(error) => {
            return rsx!(DevErrorPage {
                route,
                message: format!("failed to access localization context: {error}"),
            });
        },
    };
    let route_language = route.locale.lang();
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
            let title = format!(
                "{} | {}",
                i18n.localize_message(&SiteChromeMessage::SiteName),
                route.page.title(&i18n)
            );
            let description = route.page.description(&i18n);

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
            message: error,
        }),
    }
}

#[server(endpoint = "static_routes")]
async fn static_routes() -> Result<Vec<String>, ServerFnError> {
    Ok(all_routes()
        .into_iter()
        .map(|route| page_href(route.locale, route.page))
        .collect())
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
