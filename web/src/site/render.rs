#![cfg_attr(not(test), allow(dead_code))]

use crate::pages::route_content;
use crate::site::constants::SITE_URL;
use crate::site::i18n::{SiteLanguage, SiteMessage};
use crate::site::routing::{PageKind, SiteRoute};
use anyhow::{Context, Result};
use es_fluent::ToFluentString as _;
use es_fluent_manager_dioxus::{GlobalLocalizerMode, ssr::SsrI18n};
use std::fmt::Write as _;

pub(crate) fn render_page(locale: SiteLanguage, page: PageKind, site_root: &str) -> Result<String> {
    let i18n = SsrI18n::try_new_with_discovered_modules_and_mode(
        locale.lang(),
        GlobalLocalizerMode::ReplaceExisting,
    )
    .context("failed to initialize the Dioxus SSR localizer")?;

    let route = SiteRoute::new(locale, page);
    let title = i18n.with_manager(|| {
        format!(
            "{} | {}",
            SiteMessage::SiteName.to_fluent_string(),
            page.title_message().to_fluent_string()
        )
    });
    let description = i18n.with_manager(|| page.description_message().to_fluent_string());
    let body = i18n.render_element(route_content(route));

    Ok(render_document(
        route,
        site_root,
        &title,
        &description,
        &body,
    ))
}

fn render_document(
    route: SiteRoute,
    site_root: &str,
    title: &str,
    description: &str,
    body: &str,
) -> String {
    let body_class = if route.page.is_fullscreen() {
        "fullscreen"
    } else {
        "standard"
    };

    format!(
        "<!doctype html>\n<html lang=\"{lang}\">\n<head>\n  <meta charset=\"utf-8\" />\n  \
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n  \
         <meta name=\"description\" content=\"{description}\" />\n  <base href=\"{site_root}\" />\n  \
         <title>{title}</title>\n  <link rel=\"stylesheet\" href=\"site.css\" />\n</head>\n\
         <body class=\"{body_class}\">\n{body}\n</body>\n</html>\n",
        lang = route.locale.html_lang(),
        description = escape_html(description),
        site_root = site_root,
        title = escape_html(title),
        body_class = body_class,
        body = body,
    )
}

pub(crate) fn render_sitemap() -> String {
    let mut entries = String::new();

    for locale in SiteLanguage::all() {
        for page in PageKind::all() {
            let route = SiteRoute::new(locale, page);
            let relative = route.output_dir();
            let suffix = if relative.is_empty() {
                String::new()
            } else {
                format!("{relative}/")
            };
            let _ = writeln!(entries, "  <url><loc>{SITE_URL}{suffix}</loc></url>");
        }
    }

    let _ = writeln!(entries, "  <url><loc>{SITE_URL}book/</loc></url>");

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{entries}</urlset>\n"
    )
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
