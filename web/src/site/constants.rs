use stayhydated_dioxus::{Project, ProjectSite};

pub(crate) const PROJECT: Project = Project::new("es-fluent", "Rust localization")
    .with_skill_command("npx skills add stayhydated/es-fluent");
pub(crate) const SITE_URL: &str = "https://stayhydated.github.io/es-fluent/";
pub(crate) const RUSTDOC_URL: &str = "https://docs.rs/es-fluent/";
pub(crate) const SOURCE_URL: &str = "https://github.com/stayhydated/es-fluent";
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn site() -> ProjectSite {
    ProjectSite::builder()
        .project(PROJECT)
        .site_url(SITE_URL)
        .rustdoc_url(RUSTDOC_URL)
        .source_url(SOURCE_URL)
        .version(VERSION)
        .site_stylesheet_path("assets/site.css")
        .build()
}
