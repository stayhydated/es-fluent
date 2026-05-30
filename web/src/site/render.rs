#![cfg_attr(not(test), allow(dead_code))]

use crate::site::constants::SITE_URL;

#[cfg(test)]
use crate::site::i18n::app_dioxus_i18n_asset_modules;
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
    let _i18n = i18n.provide_context();
    children
}

#[cfg(test)]
pub(crate) fn render_route_body(route: SiteRoute) -> Result<String> {
    let runtime = SsrI18nRuntime::new(app_dioxus_i18n_asset_modules());
    let i18n = runtime
        .request_blocking(route.locale.lang())
        .context("failed to initialize the Dioxus SSR localizer")?;

    Ok(i18n.render_element(rsx! {
        SsrI18nProvider {
            i18n: i18n.clone(),
            {crate::pages::route_content(route)}
        }
    }))
}

pub(crate) fn render_sitemap() -> String {
    let mut paths = crate::site::routing::all_routes()
        .into_iter()
        .map(|route| route.path())
        .collect::<Vec<_>>();
    paths.extend([
        "/book/".to_string(),
        "/llms.txt".to_string(),
        "/llms-full.txt".to_string(),
    ]);

    stayhydated_site::sitemap::render(SITE_URL, paths)
}
