pub(super) use super::super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
pub(super) use super::{
    command::{TreeArgs, run_tree},
    json::{build_crate_tree_json, build_file_tree_json},
    links::{RustLinkIndex, TreeLinkMode, collect_rust_link_indexes},
    source_map::{FtlSourceMap, SourcePosition},
    text::{TreeRenderer, print_crate_tree},
    validation::relative_tree_message,
};
pub(super) use crate::{
    core::{CliError, CrateInfo},
    utils::ui,
};
pub(super) use clap::Parser;
pub(super) use fluent_syntax::{ast, parser};
pub(super) use std::{fs, path::Path};
pub(super) use treelog::Tree;

pub(super) fn parse_ftl(content: &str) -> ast::Resource<String> {
    parser::parse(content.to_string()).unwrap()
}

pub(super) fn get_message<'a>(
    resource: &'a ast::Resource<String>,
    id: &str,
) -> Option<&'a ast::Message<String>> {
    resource.body.iter().find_map(|entry| {
        if let ast::Entry::Message(msg) = entry
            && msg.id.name == id
        {
            return Some(msg);
        }
        None
    })
}

pub(super) fn renderer(show_attributes: bool, show_variables: bool) -> TreeRenderer<'static> {
    TreeRenderer::new(
        show_attributes,
        show_variables,
        ui::Ui::terminal_links_enabled(),
        TreeLinkMode::Rust,
        None,
    )
}

pub(super) fn position(line: usize, column: usize) -> SourcePosition {
    SourcePosition { line, column }
}

pub(super) fn create_workspace_with_tree_data() -> tempfile::TempDir {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("i18n/en/test-app")).expect("create i18n dirs");
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\n    name = \"test-app\"\n    version = \"0.1.0\"\n    edition = \"2024\"\n    ",
    )
    .expect("write Cargo.toml");
    fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
    fs::write(
        temp.path().join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write i18n.toml");
    fs::write(
        temp.path().join("i18n/en/test-app.ftl"),
        "hello = Hello { $name }\n    .title = Title { $name }\n-term = Term Value\n",
    )
    .expect("write main ftl");
    fs::write(
        temp.path().join("i18n/en/test-app/ui.ftl"),
        "button = Click\n",
    )
    .expect("write namespaced ftl");
    temp
}

pub(super) fn crate_info_from_temp(temp: &tempfile::TempDir) -> CrateInfo {
    CrateInfo {
        name: es_fluent_runner::PackageName::try_new("test-app").expect("valid package name"),
        manifest_dir: crate::core::ManifestDir::from_discovered(temp.path().to_path_buf()),
        src_dir: crate::core::SourceDir::from_discovered(temp.path().join("src")),
        library_target_path: None,
        custom_build_target_path: None,
        i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
            temp.path().join("i18n.toml"),
        ),
        ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
            temp.path().join("i18n/en"),
        ),
        has_lib_rs: true,
        fluent_features: Vec::new(),
    }
}

mod command_integration;
mod parser_links;
