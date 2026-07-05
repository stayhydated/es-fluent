use askama::Template;
use es_fluent_runner::{PackageName, RustModulePrefix};

/// Template for the generated .gitignore
#[derive(Template)]
#[template(path = ".gitignore.jinja", escape = "none")]
pub struct GitignoreTemplate;

// --- Monolithic temp crate templates ---

/// Crate dependency info for monolithic template
#[derive(Clone)]
pub struct MonolithicCrateDep<'a> {
    pub name: &'a PackageName,
    pub path: String,
    pub ident: RustModulePrefix,
    pub has_features: bool,
    pub features: &'a [String],
}

/// Template for the monolithic main.rs
#[derive(Template)]
#[template(path = "monolithic_main.rs.jinja", escape = "none")]
pub struct MonolithicMainRsTemplate<'a> {
    pub crates: Vec<MonolithicCrateDep<'a>>,
}
