//! Watch command implementation.

use super::common::{WorkspaceArgs, WorkspaceCrates, validate_generation_paths};
use crate::core::{CliError, FluentParseMode};
use crate::utils::ui;
use clap::Parser;

/// Arguments for the watch command.
#[derive(Parser)]
pub struct WatchArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for repeated FTL generation; aggressive overwrites existing translations.
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,
}

/// Run the watch command.
pub fn run_watch(args: WatchArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::Ui::print_watch_header) {
        return workspace.require_non_empty_selection();
    }
    workspace.require_all_crates_valid()?;
    validate_generation_paths(&workspace.valid, true)?;

    crate::tui::watch_all(&workspace.crates, &workspace.workspace_info, &args.mode)
        .map_err(CliError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_err as fs;

    #[test]
    fn run_watch_errors_when_package_filter_matches_nothing() {
        let temp = crate::test_fixtures::create_test_crate_workspace_without_ftl();

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            mode: FluentParseMode::default(),
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-crate"))
        );
    }

    #[test]
    fn run_watch_fails_when_discovered_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("library target"))
        );
    }

    #[test]
    fn run_watch_fails_when_any_selected_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_mixed_library_and_binary_i18n_workspace();

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
        });

        assert!(matches!(result, Err(CliError::Other(message)) if message.contains("'bin-app'")));
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

    #[test]
    fn run_watch_rejects_fallback_locale_path_as_file_before_runner_setup() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback dir");
        fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("fallback locale path 'en'"))
        );
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "watch should reject invalid generation paths before runner setup"
        );
    }

    #[test]
    fn run_watch_rejects_assets_dir_path_as_file_before_runner_setup() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("assets_dir"))
        );
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "watch should reject invalid generation paths before runner setup"
        );
    }

    #[test]
    fn run_watch_propagates_watch_all_setup_error_for_discovered_crate() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let metadata_path = temp.path().join(".es-fluent");
        if metadata_path.is_dir() {
            fs::remove_dir_all(&metadata_path).expect("remove metadata dir");
        }
        fs::write(&metadata_path, "not a directory").expect("write metadata sentinel");

        let result = run_watch(WatchArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
        });

        assert!(result.is_err());
    }
}
