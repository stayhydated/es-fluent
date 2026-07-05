#![cfg_attr(not(test), allow(dead_code))]

use crate::site::constants::SITE_URL;
use stayhydated_site::routing::SiteUrl;

pub(crate) fn render_sitemap() -> String {
    let paths = crate::site::routing::all_routes()
        .into_iter()
        .map(|route| route.path())
        .collect::<Vec<_>>();

    stayhydated_site::sitemap::render_project(&SiteUrl::new(SITE_URL), paths)
}
