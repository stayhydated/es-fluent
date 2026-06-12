mod components;
mod pages;
mod site;

pub use site::app::App;
use std::path::Path;

pub fn sitemap_xml() -> String {
    site::render::render_sitemap()
}

pub fn cleanup_generated_route_cache(public_dir: &Path) -> std::io::Result<()> {
    site::routing::cleanup_generated_route_cache(public_dir)
}

pub fn mark_generated_route_cache(public_dir: &Path) -> std::io::Result<()> {
    site::routing::mark_generated_route_cache(public_dir)
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
            "Define messages in Rust. Generate Fluent files. Use them in embedded, Bevy, and Dioxus apps."
        ));
        assert!(html.contains("href=\"/demos/\""));
        assert!(html.contains("href=\"/book/\""));
        assert!(html.contains("href=\"https://crates.io/crates/es-fluent-manager-dioxus\""));
        assert!(html.contains("Skill install command"));
        assert!(html.contains("skills"));
        assert!(html.contains("stayhydated"));
        assert!(html.contains("Project selector"));
        assert!(html.contains("Rust localization"));
        assert!(html.contains(">es-fluent<"));
        assert!(!html.contains("/koruma/"));
        assert!(!html.contains("Rust validation"));
        assert!(!html.contains("Project index"));
    }

    #[test]
    #[serial]
    fn localizes_language_select_labels() {
        let runtime = es_fluent_manager_dioxus::ssr::SsrI18nRuntime::discovered();
        let i18n = runtime
            .request_blocking(SiteLanguage::EnUs.lang())
            .expect("SSR i18n should initialize");

        let english = i18n.localize_message(&SiteLanguage::EnUs);
        let chinese = i18n.localize_message(&SiteLanguage::ZhCn);

        assert!(!english.starts_with("es-fluent-lang-"));
        assert!(!chinese.starts_with("es-fluent-lang-"));
        assert_ne!(english, chinese);
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
        assert!(html.contains("href=\"/zh/gpui-example/\""));
        assert!(html.contains("Project selector"));
        assert!(html.contains("Rust localization"));
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
    #[serial]
    fn renders_gpui_pages_with_relative_demo_bundle_paths() {
        let english = crate::site::render::render_route_body(SiteRoute::new(
            SiteLanguage::EnUs,
            PageKind::Gpui,
        ))
        .expect("page should render");
        assert!(english.contains("src=\"../gpui-demo/\""));

        let chinese = crate::site::render::render_route_body(SiteRoute::new(
            SiteLanguage::ZhCn,
            PageKind::Gpui,
        ))
        .expect("page should render");
        assert!(chinese.contains("src=\"../../gpui-demo/\""));
    }

    #[test]
    fn computes_site_root_prefixes() {
        assert_eq!(
            crate::site::routing::site_root_prefix(&stayhydated_site::routing::OutputDir::new("")),
            "./"
        );
        assert_eq!(
            crate::site::routing::site_root_prefix(&stayhydated_site::routing::OutputDir::new(
                "demos"
            )),
            "../"
        );
        assert_eq!(
            crate::site::routing::site_root_prefix(&stayhydated_site::routing::OutputDir::new(
                "zh/demos"
            )),
            "../../"
        );
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
            crate::site::routing::site_route_from_path("/gpui-example/"),
            SiteRoute::new(SiteLanguage::EnUs, PageKind::Gpui)
        );
        assert_eq!(
            crate::site::routing::site_route_from_path("/unknown"),
            SiteRoute::new(SiteLanguage::EnUs, PageKind::Home)
        );
    }

    #[test]
    fn parses_localized_site_routes() {
        assert_eq!(
            crate::site::routing::site_route_from_path_with_base_path(
                "/your_repo/zh/gpui-example/",
                Some("your_repo")
            ),
            SiteRoute::new(SiteLanguage::ZhCn, PageKind::Gpui)
        );
    }

    #[test]
    fn sitemap_includes_generated_static_outputs() {
        let sitemap = crate::sitemap_xml();

        assert!(sitemap.contains("<loc>https://stayhydated.github.io/es-fluent/</loc>"));
        assert!(sitemap.contains("<loc>https://stayhydated.github.io/es-fluent/book/</loc>"));
        assert!(sitemap.contains("<loc>https://stayhydated.github.io/es-fluent/llms.txt</loc>"));
        assert!(
            sitemap.contains("<loc>https://stayhydated.github.io/es-fluent/llms-full.txt</loc>")
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
        fs::create_dir_all(public_dir.join("gpui-demo")).expect("create gpui-demo dir");
        fs::write(public_dir.join("gpui-demo").join("index.html"), "gpui")
            .expect("write gpui-demo");
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
        assert!(public_dir.join("gpui-demo").join("index.html").exists());
        assert!(public_dir.join("assets").join("site.css").exists());
    }
}
