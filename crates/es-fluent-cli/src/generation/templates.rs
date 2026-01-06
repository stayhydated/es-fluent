use askama::Template;

/// Template for the .cargo/config.toml (target-dir setting)
#[derive(Template)]
#[template(path = "config.toml.jinja", escape = "none")]
pub struct ConfigTomlTemplate<'a> {
    pub target_dir: &'a str,
}

/// Template for the generated .gitignore
#[derive(Template)]
#[template(path = ".gitignore.jinja", escape = "none")]
pub struct GitignoreTemplate;

// --- Monolithic temp crate templates ---

/// Crate dependency info for monolithic template
#[derive(Clone)]
pub struct MonolithicCrateDep<'a> {
    pub name: &'a str,
    pub path: String,
    pub ident: String,
    pub has_features: bool,
    pub features: &'a [String],
}

/// Template for the monolithic Cargo.toml (depends on ALL workspace crates)
#[derive(Template)]
#[template(path = "MonolithicCargo.toml.jinja", escape = "none")]
pub struct MonolithicCargoTomlTemplate<'a> {
    pub crates: Vec<MonolithicCrateDep<'a>>,
    pub es_fluent_dep: &'a str,
    pub es_fluent_cli_helpers_dep: &'a str,
}

/// Template for the monolithic main.rs
#[derive(Template)]
#[template(path = "monolithic_main.rs.jinja", escape = "none")]
pub struct MonolithicMainRsTemplate<'a> {
    pub crates: Vec<MonolithicCrateDep<'a>>,
}
