pub(super) use super::super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
pub(super) use super::{
    execution::run_sync,
    locale,
    report::{collect_affected_locale_targets, relative_sync_message},
    selection::SyncArgs,
};
pub(super) use crate::core::CliError;
pub(super) use fluent_syntax::parser;
pub(super) use fs_err as fs;
pub(super) use std::collections::HashSet;

pub(super) fn write_sync_workspace_crate(root: &std::path::Path, name: &str, fallback: &str) {
    fs::create_dir_all(root.join(name).join("src")).expect("create src");
    fs::create_dir_all(root.join(name).join("i18n/en")).expect("create fallback locale");
    fs::write(
        root.join(name).join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n"
        ),
    )
    .expect("write manifest");
    fs::write(root.join(name).join("src/lib.rs"), "pub fn marker() {}\n").expect("write lib");
    fs::write(
        root.join(name).join("i18n.toml"),
        "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
    )
    .expect("write config");
    fs::write(
        root.join(name).join("i18n/en").join(format!("{name}.ftl")),
        fallback,
    )
    .expect("write fallback ftl");
}

mod create;
mod multi_crate;
mod rollback;
mod selection;
