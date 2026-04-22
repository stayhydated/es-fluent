pub(crate) const SITE_URL: &str = "https://stayhydated.github.io/es-fluent/";
pub(crate) const README_URL: &str =
    "https://github.com/stayhydated/es-fluent/blob/master/README.md";
pub(crate) const CRATES_URL: &str = "https://github.com/stayhydated/es-fluent/tree/master/crates";
pub(crate) const DIOXUS_EXAMPLE_URL: &str =
    "https://github.com/stayhydated/es-fluent/tree/master/examples/dioxus-example";
pub(crate) const INSTALL_SNIPPET: &str = r#"[dependencies]
es-fluent = { version = "*", features = ["derive"] }
unic-langid = "*"

# Pick one runtime manager:
es-fluent-manager-embedded = "*"
es-fluent-manager-bevy = "*"
es-fluent-manager-dioxus = { version = "*", features = ["web"] }"#;
