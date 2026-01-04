use askama::Template;

/// Template for the generated Cargo.toml
#[derive(Template)]
#[template(path = "Cargo.toml.jinja", escape = "none")]
pub struct CargoTomlTemplate<'a> {
    pub crate_name: &'a str,
    pub parent_crate_name: &'a str,
    pub es_fluent_dep: &'a str,
    pub es_fluent_cli_helpers_dep: &'a str,
    /// Whether the crate specifies fluent features in i18n.toml
    pub has_fluent_features: bool,
    /// The fluent feature names (only meaningful if has_fluent_features is true)
    pub fluent_features: &'a [String],
}

/// Template for the ftl generation command main.rs
#[derive(Template)]
#[template(path = "generate.rs.jinja", escape = "none")]
pub struct GenerateRsTemplate<'a> {
    pub crate_ident: &'a str,
    pub i18n_toml_path: &'a str,
    pub crate_name: &'a str,
}

/// Template for the check command main.rs
#[derive(Template)]
#[template(path = "check.rs.jinja", escape = "none")]
pub struct CheckRsTemplate<'a> {
    pub crate_ident: &'a str,
    pub crate_name: &'a str,
}

/// Template for the generated .gitignore
#[derive(Template)]
#[template(path = ".gitignore.jinja", escape = "none")]
pub struct GitignoreTemplate;
