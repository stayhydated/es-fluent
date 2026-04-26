#![cfg_attr(not(test), allow(dead_code))]

use crate::site::constants::SITE_URL;
use crate::site::routing::all_routes;
use std::fmt::Write as _;

#[cfg(test)]
use crate::pages::route_content;
#[cfg(test)]
use crate::site::routing::SiteRoute;
#[cfg(test)]
use anyhow::{Context, Result};
#[cfg(test)]
use es_fluent_manager_dioxus::ssr::SsrI18nRuntime;

#[cfg(test)]
pub(crate) fn render_route_body(route: SiteRoute) -> Result<String> {
    let runtime = SsrI18nRuntime::install()
        .context("failed to install the Dioxus SSR process-global localizer")?;
    let i18n = runtime
        .request(route.locale.lang())
        .context("failed to initialize the Dioxus SSR localizer")?;

    i18n.render_element(route_content(route))
        .context("failed to render route with the Dioxus SSR localizer")
}

pub(crate) fn render_sitemap() -> String {
    let mut entries = String::new();

    for route in all_routes() {
        let path = route.path();
        let url = if path == "/" {
            SITE_URL.to_string()
        } else {
            format!("{SITE_URL}{}", path.trim_start_matches('/'))
        };
        let _ = writeln!(entries, "  <url><loc>{url}</loc></url>");
    }

    let _ = writeln!(entries, "  <url><loc>{SITE_URL}book/</loc></url>");

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{entries}</urlset>\n"
    )
}
