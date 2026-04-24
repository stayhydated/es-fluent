mod components;
mod pages;
mod site;

pub use site::app::App;

pub fn sitemap_xml() -> String {
    site::render::render_sitemap()
}

#[cfg(test)]
mod tests {
    use crate::site::i18n::SiteLanguage;
    use crate::site::render::render_route_body;
    use crate::site::routing::{
        PageKind, SiteRoute, site_root_prefix, site_route_from_path,
        site_route_from_path_with_base_path,
    };
    use serial_test::serial;

    #[test]
    #[serial]
    fn renders_english_home_page() {
        let html = render_route_body(SiteRoute::new(SiteLanguage::EnUs, PageKind::Home))
            .expect("page should render");
        assert!(html.contains(
            "Define Fluent messages from Rust types, validate locale assets in CI, and reuse the same message model across embedded, Bevy, and Dioxus runtimes."
        ));
        assert!(html.contains("href=\"/demos/\""));
        assert!(html.contains("href=\"/book/\""));
        assert!(html.contains("href=\"https://crates.io/crates/es-fluent-manager-dioxus\""));
    }

    #[test]
    #[serial]
    fn renders_french_demos_page() {
        let html = render_route_body(SiteRoute::new(SiteLanguage::FrFr, PageKind::Demos))
            .expect("page should render");
        assert!(html.contains("href=\"/fr/bevy-example/\""));
        assert!(html.contains("Lancer la démo"));
        assert!(!html.contains("Démo navigateur en direct"));
    }

    #[test]
    #[serial]
    fn renders_bevy_pages_with_relative_demo_bundle_paths() {
        let english = render_route_body(SiteRoute::new(SiteLanguage::EnUs, PageKind::Bevy))
            .expect("page should render");
        assert!(english.contains("src=\"../bevy-demo/\""));

        let french = render_route_body(SiteRoute::new(SiteLanguage::FrFr, PageKind::Bevy))
            .expect("page should render");
        assert!(french.contains("src=\"../../bevy-demo/\""));
    }

    #[test]
    fn computes_site_root_prefixes() {
        assert_eq!(site_root_prefix(""), "./");
        assert_eq!(site_root_prefix("demos"), "../");
        assert_eq!(site_root_prefix("fr/demos"), "../../");
    }

    #[test]
    fn parses_site_routes() {
        assert_eq!(
            site_route_from_path_with_base_path("/your_repo/fr/demos/", Some("your_repo")),
            SiteRoute::new(SiteLanguage::FrFr, PageKind::Demos)
        );
        assert_eq!(
            site_route_from_path("/bevy-example/"),
            SiteRoute::new(SiteLanguage::EnUs, PageKind::Bevy)
        );
        assert_eq!(
            site_route_from_path("/unknown"),
            SiteRoute::new(SiteLanguage::EnUs, PageKind::Home)
        );
    }
}
