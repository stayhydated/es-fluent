//! Watch command implementation.

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, FluentParseMode};
use crate::tui::watch_all;
use crate::utils::ui;
use clap::Parser;

/// Arguments for the watch command.
#[derive(Parser)]
pub struct WatchArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,
}

/// Run the watch command.
pub fn run_watch(args: WatchArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    watch_all(&workspace.crates, &workspace.workspace_info, &args.mode).map_err(CliError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_crate_workspace() -> tempfile::TempDir {
        let temp = tempdir().unwrap();

        fs::create_dir_all(temp.path().join("src")).unwrap();
        fs::create_dir_all(temp.path().join("i18n/en")).unwrap();
        fs::write(
            temp.path().join("Cargo.toml"),
            r#"[package]
name = "test-app"
version = "0.1.0"
edition = "2024"
"#,
        )
        .unwrap();
        fs::write(temp.path().join("src/lib.rs"), "pub struct Demo;\n").unwrap();
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();

        temp
    }

    #[test]
    fn run_watch_returns_ok_when_package_filter_matches_nothing() {
        let temp = create_test_crate_workspace();

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            mode: FluentParseMode::default(),
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_watch_returns_error_for_invalid_path() {
        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(std::path::PathBuf::from("/definitely/missing/path")),
                package: None,
            },
            mode: FluentParseMode::default(),
        });

        assert!(result.is_err());
    }
}
