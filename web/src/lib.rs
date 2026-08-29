mod pages;
mod site;

pub use site::app::App;

pub fn route_manifest() -> stayhydated_site::SiteRouteManifest {
    site::constants::site()
        .route_manifest::<site::routing::AppRoute>()
        .with_static_paths([
            "/bevy-demo/",
            "/gpui-demo/",
            "/typescript-demo/",
            "/solid-demo/",
            "/react-demo/",
            "/expo-demo/",
        ])
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
            [
                "/",
                "/demos/",
                "/dioxus-example/",
                "/bevy-example/",
                "/gpui-example/",
                "/typescript-examples/",
            ]
        );
        assert!(
            [
                "/bevy-demo/",
                "/gpui-demo/",
                "/typescript-demo/",
                "/solid-demo/",
                "/react-demo/",
                "/expo-demo/",
            ]
            .into_iter()
            .all(|expected| {
                manifest
                    .static_paths()
                    .iter()
                    .any(|path| path.as_str() == expected)
            })
        );
        assert_eq!(manifest.site_url().as_str(), site::constants::SITE_URL);
        assert_eq!(
            site::constants::PROJECT.skill_command(),
            Some("npx skills add stayhydated/es-fluent")
        );
    }
}
