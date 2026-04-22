mod components;
mod pages;
mod site;

#[cfg(feature = "web")]
pub use site::app::DevApp;
pub use site::build::{build_site, run};

#[cfg(test)]
mod tests {
    use crate::site::i18n::SiteLanguage;
    use crate::site::render::render_page;
    use crate::site::routing::{PageKind, SiteRoute, site_root_prefix, site_route_from_path};
    use serial_test::serial;

    #[test]
    #[serial]
    fn renders_english_home_page() {
        let html =
            render_page(SiteLanguage::EnUs, PageKind::Home, "./").expect("page should render");
        assert!(html.contains("Ship localized Rust UIs without drifting out of sync."));
        assert!(html.contains("href=\"/es-fluent/demos/\""));
        assert!(html.contains("href=\"/es-fluent/book/\""));
    }

    #[test]
    #[serial]
    fn renders_french_demos_page() {
        let html =
            render_page(SiteLanguage::FrFr, PageKind::Demos, "../../").expect("page should render");
        assert!(html.contains("Démos navigateur et pistes d’intégration"));
        assert!(html.contains("href=\"/es-fluent/fr/bevy-example/\""));
        assert!(html.contains("Ouvrir la source"));
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
            site_route_from_path("/es-fluent/fr/demos/"),
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
