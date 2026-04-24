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
use es_fluent_manager_dioxus::{GlobalLocalizerMode, ssr::SsrI18n};

#[cfg(test)]
pub(crate) fn render_route_body(route: SiteRoute) -> Result<String> {
    let i18n = SsrI18n::try_new_with_discovered_modules_and_mode(
        route.locale.lang(),
        GlobalLocalizerMode::ReplaceExisting,
    )
    .context("failed to initialize the Dioxus SSR localizer")?;

    Ok(i18n.render_element(route_content(route)))
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
