mod components;
mod pages;
mod site;

pub use site::app::App;

pub fn sitemap_xml() -> String {
    site::render::render_sitemap()
}

pub fn route_paths() -> Vec<String> {
    site::routing::all_routes()
        .into_iter()
        .map(|route| route.path().into_string())
        .collect()
}
