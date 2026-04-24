use crate::pages::{DevErrorPage, route_content};
use crate::site::i18n::{
    BevyPageMessage, DemosPageMessage, HomeHeroMessage, PageMetadataMessage, SiteChromeMessage,
    SiteLanguage,
};
use dioxus::cli_config;
use dioxus::prelude::server;
use dioxus::prelude::{Meta, Routable, ServerFnError, Title, VNode};
use dioxus::router as dioxus_router;
use dioxus_core::Element;
use dioxus_core::use_hook;
use dioxus_core_macro::{Props, component, rsx};
#[allow(unused_imports)]
use dioxus_html as dioxus_elements;
use es_fluent::ToFluentString as _;
use es_fluent_manager_dioxus::ManagedI18n;
use es_fluent_manager_dioxus::{GlobalLocalizerMode, use_provide_i18n_with_mode};
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

    pub(crate) fn title(self) -> String {
        match self {
            Self::Home => PageMetadataMessage::HomeTitle.to_fluent_string(),
            Self::Demos => PageMetadataMessage::DemosTitle.to_fluent_string(),
            Self::Bevy => PageMetadataMessage::BevyTitle.to_fluent_string(),
        }
    }

    pub(crate) fn description(self) -> String {
        match self {
            Self::Home => HomeHeroMessage::Body.to_fluent_string(),
            Self::Demos => DemosPageMessage::BevyBody.to_fluent_string(),
            Self::Bevy => BevyPageMessage::Lead.to_fluent_string(),
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

pub(crate) fn cleanup_generated_route_cache(public_dir: &Path) -> std::io::Result<()> {
    if !public_dir.exists() {
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
        let name = name.to_string_lossy();
        if generated_top_level_dirs.contains(name.as_ref()) || is_static_public_dir(name.as_ref()) {
            continue;
        }

        if directory_contains_generated_html(&entry.path())? {
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

fn is_static_public_dir(name: &str) -> bool {
    matches!(name, "assets" | "bevy-demo" | "book" | "wasm")
}

fn directory_contains_generated_html(dir: &Path) -> std::io::Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            if directory_contains_generated_html(&path)? {
                return Ok(true);
            }
        } else if path.file_name().and_then(|name| name.to_str()) == Some("index.html") {
            return Ok(true);
        }
    }

    Ok(false)
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
                SiteChromeMessage::SiteName.to_fluent_string(),
                route.page.title()
            );
            let description = route.page.description();

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
