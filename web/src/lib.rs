mod components;
mod pages;
mod site;

pub use site::app::App;
use std::path::Path;

pub fn sitemap_xml() -> String {
    site::render::render_sitemap()
}

pub fn cleanup_generated_route_cache(public_dir: impl AsRef<Path>) -> std::io::Result<()> {
    site::routing::cleanup_generated_route_cache(public_dir.as_ref())
}

pub fn mark_generated_route_cache(public_dir: impl AsRef<Path>) -> std::io::Result<()> {
    site::routing::mark_generated_route_cache(public_dir.as_ref())
}

#[cfg(test)]
mod tests {
    use crate::site::i18n::SiteLanguage;
    use crate::site::routing::{PageKind, SiteRoute};
    use serial_test::serial;
    use std::fs;

    #[test]
    #[serial]
    fn renders_english_home_page() {
        let html = crate::site::render::render_route_body(SiteRoute::new(
            SiteLanguage::EnUs,
            PageKind::Home,
        ))
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
    fn renders_simplified_chinese_demos_page() {
        let html = crate::site::render::render_route_body(SiteRoute::new(
            SiteLanguage::ZhCn,
            PageKind::Demos,
        ))
        .expect("page should render");
        assert!(html.contains("href=\"/zh/bevy-example/\""));
        assert!(html.contains("打开演示"));
        assert!(!html.contains("Lancer la démo"));
    }

    #[test]
    #[serial]
    fn renders_bevy_pages_with_relative_demo_bundle_paths() {
        let english = crate::site::render::render_route_body(SiteRoute::new(
            SiteLanguage::EnUs,
            PageKind::Bevy,
        ))
        .expect("page should render");
        assert!(english.contains("src=\"../bevy-demo/\""));

        let chinese = crate::site::render::render_route_body(SiteRoute::new(
            SiteLanguage::ZhCn,
            PageKind::Bevy,
        ))
        .expect("page should render");
        assert!(chinese.contains("src=\"../../bevy-demo/\""));
    }

    #[test]
    fn computes_site_root_prefixes() {
        assert_eq!(crate::site::routing::site_root_prefix(""), "./");
        assert_eq!(crate::site::routing::site_root_prefix("demos"), "../");
        assert_eq!(crate::site::routing::site_root_prefix("zh/demos"), "../../");
    }

    #[test]
    fn parses_site_routes() {
        assert_eq!(
            crate::site::routing::site_route_from_path_with_base_path(
                "/your_repo/zh/demos/",
                Some("your_repo")
            ),
            SiteRoute::new(SiteLanguage::ZhCn, PageKind::Demos)
        );
        assert_eq!(
            crate::site::routing::site_route_from_path("/bevy-example/"),
            SiteRoute::new(SiteLanguage::EnUs, PageKind::Bevy)
        );
        assert_eq!(
            crate::site::routing::site_route_from_path("/unknown"),
            SiteRoute::new(SiteLanguage::EnUs, PageKind::Home)
        );
    }

    #[test]
    fn cleans_generated_route_cache_without_touching_static_assets() {
        let temp = tempfile::tempdir().expect("tempdir");
        let public_dir = temp.path();

        fs::write(public_dir.join("index.html"), "root").expect("write root index");
        fs::write(public_dir.join("404.html"), "not found").expect("write root 404");
        fs::create_dir_all(public_dir.join("demos")).expect("create demos dir");
        fs::write(public_dir.join("demos").join("index.html"), "stale demos")
            .expect("write demos index");
        fs::create_dir_all(public_dir.join("zh").join("demos")).expect("create zh demos dir");
        fs::write(
            public_dir.join("zh").join("demos").join("index.html"),
            "stale zh demos",
        )
        .expect("write zh demos index");
        fs::create_dir_all(public_dir.join("fr").join("demos")).expect("create fr demos dir");
        fs::write(public_dir.join("fr").join("index.html"), "stale fr index")
            .expect("write fr index");
        fs::write(
            public_dir.join("fr").join("demos").join("index.html"),
            "stale fr demos",
        )
        .expect("write fr demos index");
        fs::create_dir_all(public_dir.join("book")).expect("create book dir");
        fs::write(public_dir.join("book").join("index.html"), "book").expect("write book");
        fs::create_dir_all(public_dir.join("bevy-demo")).expect("create bevy-demo dir");
        fs::write(public_dir.join("bevy-demo").join("index.html"), "bevy")
            .expect("write bevy-demo");
        fs::create_dir_all(public_dir.join("assets")).expect("create assets dir");
        fs::write(public_dir.join("assets").join("site.css"), "body {}").expect("write asset");

        crate::mark_generated_route_cache(public_dir).expect("mark route cache");
        crate::cleanup_generated_route_cache(public_dir).expect("cleanup route cache");

        assert!(!public_dir.join("index.html").exists());
        assert!(!public_dir.join("404.html").exists());
        assert!(!public_dir.join("demos").exists());
        assert!(!public_dir.join("zh").exists());
        assert!(!public_dir.join("fr").exists());
        assert!(public_dir.join("book").join("index.html").exists());
        assert!(public_dir.join("bevy-demo").join("index.html").exists());
        assert!(public_dir.join("assets").join("site.css").exists());
    }
}
