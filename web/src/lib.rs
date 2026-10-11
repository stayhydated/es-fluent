mod pages;
mod site;

pub use site::app::App;

pub fn route_manifest() -> stayhydated_site::SiteRouteManifest {
    site::constants::site()
        .route_manifest::<site::routing::AppRoute>()
        .with_static_paths(["/bevy-demo/"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_manifest_uses_every_application_route() {
        let manifest = route_manifest();

        assert_eq!(
            manifest
                .application_paths()
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>(),
            ["/", "/demos/", "/dioxus-example/", "/bevy-example/"]
        );
        assert!(
            manifest
                .static_paths()
                .iter()
                .any(|path| path.as_str() == "/bevy-demo/")
        );
        assert_eq!(manifest.site_url().as_str(), site::constants::SITE_URL);
        assert_eq!(
            site::constants::PROJECT.skill_command(),
            Some("npx skills add stayhydated/es-fluent")
        );
    }
}
