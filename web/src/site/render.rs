#![cfg_attr(not(test), allow(dead_code))]

use crate::site::constants::SITE_URL;
use crate::site::routing::all_routes;
use std::fmt::Write as _;

#[cfg(test)]
use crate::pages::route_content;
#[cfg(test)]
use crate::site::routing::SiteRoute;
#[cfg(test)]
use anyhow::{Context as _, Result};
#[cfg(test)]
use dioxus::prelude::*;
#[cfg(test)]
use es_fluent_manager_dioxus::ssr::{SsrI18n, SsrI18nRuntime};

#[cfg(test)]
#[component]
fn SsrI18nProvider(i18n: SsrI18n, children: Element) -> Element {
    i18n.provide_context()
        .expect("SSR i18n context should be ready");
    children
}

#[cfg(test)]
pub(crate) fn render_route_body(route: SiteRoute) -> Result<String> {
    let runtime = SsrI18nRuntime::new();
    let i18n = runtime
        .request(route.locale.lang())
        .context("failed to initialize the Dioxus SSR localizer")?;

    Ok(i18n.render_element(rsx! {
        SsrI18nProvider {
            i18n: i18n.clone(),
            {route_content(route)}
        }
    }))
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
