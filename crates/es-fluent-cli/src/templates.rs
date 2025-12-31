use askama::Template;

/// Template for the generated Cargo.toml
#[derive(Template)]
#[template(path = "Cargo.toml.jinja", escape = "none")]
pub struct CargoTomlTemplate<'a> {
    pub crate_name: &'a str,
    pub parent_crate_name: &'a str,
    pub es_fluent_dep: &'a str,
    /// Whether the crate specifies a fluent feature in i18n.toml
    pub has_fluent_feature: bool,
    /// The fluent feature name (only meaningful if has_fluent_feature is true)
    pub fluent_feature: &'a str,
}

/// Template for the generated main.rs
#[derive(Template)]
#[template(path = "main.rs.jinja", escape = "none")]
pub struct MainRsTemplate<'a> {
    pub crate_ident: &'a str,
    pub i18n_toml_path: &'a str,
    pub parse_mode: &'a str,
    pub crate_name: &'a str,
}

/// Template for the generated .gitignore
#[derive(Template)]
#[template(path = ".gitignore.jinja", escape = "none")]
pub struct GitignoreTemplate;
