//! Generate command implementation.

use super::common::{GenerationVerb, WorkspaceArgs};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use clap::Parser;

/// Arguments for the generate command.
#[derive(Parser)]
pub struct GenerateArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,

    /// Dry run - show what would be generated without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,
}

/// Run the generate command.
pub fn run_generate(args: GenerateArgs) -> Result<(), CliError> {
    super::common::run_generation_command(
        args.workspace,
        GenerationAction::Generate {
            mode: args.mode,
            dry_run: args.dry_run,
        },
        args.force_run,
        args.dry_run,
        GenerationVerb::Generate,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::FakeRunnerBehavior;
    use fs_err as fs;

    #[test]
    fn run_generate_errors_when_package_filter_matches_nothing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-crate"))
        );
    }

    #[test]
    fn run_generate_fails_when_discovered_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();
        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("library target"))
        );
    }

    #[test]
    fn run_generate_fails_when_any_selected_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_mixed_library_and_binary_i18n_workspace();
        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(matches!(result, Err(CliError::Other(message)) if message.contains("'bin-app'")));
    }

    #[test]
    fn run_generate_errors_when_fallback_locale_path_is_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback dir");
        fs::write(temp.path().join("i18n/en"), "not a directory\n").expect("write fallback file");

        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("fallback locale path 'en'"))
        );
    }

    #[test]
    fn run_generate_errors_when_output_dir_parent_path_is_file_before_runner_setup() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::write(
            temp.path().join("Cargo.toml"),
            crate::test_fixtures::CARGO_TOML,
        )
        .expect("write manifest");
        fs::write(temp.path().join("src/lib.rs"), crate::test_fixtures::LIB_RS).expect("write lib");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n/locales\"\n",
        )
        .expect("write i18n config");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write blocker");

        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("path component is not a directory") && message.contains("i18n"))
        );
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "generate should reject blocked output parents before runner metadata"
        );
        assert!(
            !temp.path().join("target").exists(),
            "generate should reject blocked output parents before Cargo runs"
        );
    }

    #[test]
    fn run_generate_errors_when_fallback_ftl_path_is_directory_before_runner_setup() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let ftl_path = temp.path().join("i18n/en/test-app.ftl");
        fs::remove_file(&ftl_path).expect("remove ftl file");
        fs::create_dir(&ftl_path).expect("create ftl directory");
        fs::write(temp.path().join("src/lib.rs"), "this is not rust\n").expect("break Rust");

        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("fallback locale FTL layout") && message.contains("Expected FTL path"))
        );
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "generate should reject invalid FTL paths before runner metadata"
        );
        assert!(
            !temp.path().join("target").exists(),
            "generate should reject invalid FTL paths before Cargo runs"
        );
    }

    #[test]
    fn run_generate_executes_with_fake_runner() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::stdout("generated\n"),
        );

        let result = run_generate(GenerateArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            mode: FluentParseMode::default(),
            dry_run: false,
            force_run: false,
        });

        assert!(result.is_ok());
    }
}
