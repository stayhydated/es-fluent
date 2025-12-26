use askama::Template;

/// Template for the generated Cargo.toml
#[derive(Template)]
#[template(path = "Cargo.toml.jinja", escape = "none")]
pub struct CargoTomlTemplate<'a> {
    pub crate_name: &'a str,
    pub parent_crate_name: &'a str,
    pub es_fluent_dep: &'a str,
}

/// Template for the generated main.rs
#[derive(Template)]
#[template(path = "main.rs.jinja", escape = "none")]
pub struct MainRsTemplate<'a> {
    pub crate_ident: &'a str,
    pub i18n_toml_path: &'a str,
    pub parse_mode: &'a str,
    pub crate_name: &'a str,
    pub crate_root: &'a str,
}

/// Template for the generated .gitignore
#[derive(Template)]
#[template(path = ".gitignore.jinja", escape = "none")]
pub struct GitignoreTemplate;
