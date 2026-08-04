//! Clean command implementation.

pub(crate) mod orphaned;

use super::common::{
    GenerationVerb, WorkspaceArgs, WorkspaceCrates, render_generation_results_with_dry_run,
    run_generation_for_crates_with_transaction, validate_generation_paths,
};
use crate::core::{CliError, GenerationAction};
use clap::Parser;

/// Arguments for the clean command.
#[derive(Parser)]
pub struct CleanArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Clean generated entries in all discovered locale directories; orphan-file cleanup
    /// still requires --orphaned.
    #[arg(long)]
    pub all_locales: bool,

    /// Dry run - show locale-file changes and orphan removals without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Also remove orphaned FTL files from non-fallback locales.
    /// Explicitly passing this flag scans non-fallback locales even without --all-locales.
    #[arg(long)]
    pub orphaned: bool,
}

/// Run the clean command.
pub fn run_clean(args: CleanArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(crate::utils::ui::Ui::print_clean_header) {
        return workspace.require_non_empty_selection();
    }
    if !args.orphaned || args.all_locales {
        workspace.require_all_crates_valid()?;
    }
    validate_generation_paths(&workspace.valid, !args.orphaned)?;
    if args.orphaned {
        orphaned::validate_orphaned_scan_setup(&workspace, true)?;
    }

    let orphaned_plan = if args.orphaned {
        Some(orphaned::plan_orphaned_files(&workspace, true)?)
    } else {
        None
    };

    if !workspace.valid.is_empty() {
        let action = GenerationAction::Clean {
            all_locales: args.all_locales,
            dry_run: args.dry_run,
        };
        let results = run_generation_for_crates_with_transaction(
            &workspace.workspace_info,
            &workspace.valid,
            &action,
            args.force_run,
            true,
            orphaned_plan
                .as_ref()
                .map(|plan| plan.transaction().clone())
                .unwrap_or_default(),
        );
        let has_errors =
            render_generation_results_with_dry_run(&results, args.dry_run, GenerationVerb::Clean);

        if has_errors {
            return Err(CliError::Other(
                "generation command failed; see diagnostics above".to_string(),
            ));
        }
    } else if let Some(plan) = &orphaned_plan
        && !args.dry_run
    {
        plan.transaction()
            .commit()
            .map_err(|error| CliError::Other(error.to_string()))?;
    }

    if let Some(plan) = &orphaned_plan {
        orphaned::render_orphaned_cleanup(plan, args.dry_run);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::FakeRunnerBehavior;
    use fs_err as fs;

    #[test]
    fn run_clean_errors_when_package_filter_matches_nothing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-crate"))
        );
    }

    #[test]
    fn run_clean_fails_when_discovered_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("library target"))
        );
    }

    #[test]
    fn run_clean_fails_when_any_selected_crate_has_no_library_target() {
        let temp = crate::test_fixtures::create_mixed_library_and_binary_i18n_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(matches!(result, Err(CliError::Other(message)) if message.contains("'bin-app'")));
    }

    #[test]
    fn run_clean_errors_when_assets_dir_path_is_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("assets_dir"))
        );
    }

    #[test]
    fn run_clean_orphaned_errors_when_assets_dir_path_is_file() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");
        fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: true,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("generation path") && message.contains("assets_dir"))
        );
    }

    #[test]
    fn run_clean_orphaned_errors_when_assets_dir_is_missing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::remove_dir_all(temp.path().join("i18n")).expect("remove assets dir");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: true,
            force_run: false,
            orphaned: true,
        });

        let Err(error) = result else {
            panic!("expected missing assets_dir error, got {result:?}");
        };
        let message = error.to_string();
        assert!(
            message.contains("i18n") && message.contains("does not exist"),
            "unexpected missing assets_dir error: {message}"
        );
    }

    #[test]
    fn run_clean_all_skips_orphan_scan_setup_without_orphaned_flag() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let args_path = temp.path().join("runner-args.txt");
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::record_args(&args_path),
        );
        fs::write(temp.path().join("i18n/fr"), "not a directory\n").expect("write locale file");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: true,
            dry_run: true,
            force_run: false,
            orphaned: false,
        });

        assert!(result.is_ok());
        let args = fs::read_to_string(args_path).expect("read recorded runner args");
        assert!(
            args.contains(r#""command":"clean""#) && args.contains(r#""all_locales":true"#),
            "clean --all-locales should run generated-entry cleanup without orphan-scan setup, got {args}"
        );
    }

    #[test]
    fn run_clean_orphaned_errors_when_fallback_missing_before_runner() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        fs::remove_dir_all(temp.path().join("i18n/en")).expect("remove fallback locale");
        fs::create_dir_all(temp.path().join("i18n/fr")).expect("create non-fallback locale");
        fs::write(
            temp.path().join("i18n/fr/test-app.ftl"),
            "hello = Bonjour\n",
        )
        .expect("write non-fallback ftl");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: true,
            force_run: false,
            orphaned: true,
        });

        let Err(error) = result else {
            panic!("expected missing fallback locale error, got {result:?}");
        };
        let message = error.to_string();
        assert!(message.contains("fallback locale directory"));
        assert!(message.contains("refusing to scan orphaned files"));
        assert!(
            !temp
                .path()
                .join(".es-fluent/metadata/test-app/clean.json")
                .exists(),
            "clean runner should not execute after orphan-scan setup errors"
        );
    }

    #[test]
    fn run_clean_orphaned_allows_file_only_scan_without_library_target() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: true,
            force_run: false,
            orphaned: true,
        });

        assert!(result.is_ok());
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "file-only orphan scans should not prepare the runner crate"
        );
        assert!(
            !temp.path().join("target").exists(),
            "file-only orphan scans should not run Cargo"
        );
    }

    #[test]
    fn run_clean_executes_with_fake_runner() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::stdout("cleaned\n"),
        );

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_clean_orphaned_errors_when_package_filter_matches_nothing() {
        let temp = crate::test_fixtures::create_test_crate_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: true,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-crate"))
        );
    }

    #[test]
    fn run_clean_orphaned_handles_workspace_without_orphans() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: true,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_clean_all_preserves_orphaned_files_without_orphaned_flag() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hola\n"),
        ]);
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        let orphan = temp.path().join("i18n/es/orphan.ftl");
        fs::write(&orphan, "orphan = Orphan\n").expect("write orphan");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: true,
            dry_run: false,
            force_run: false,
            orphaned: false,
        });

        assert!(result.is_ok());
        assert!(
            orphan.exists(),
            "clean --all-locales should leave file orphans for an explicit --orphaned run"
        );
    }

    #[test]
    fn run_clean_orphaned_scans_non_fallback_locales_without_all() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hola\n"),
        ]);
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::silent_success(),
        );
        let orphan = temp.path().join("i18n/es/orphan.ftl");
        fs::write(&orphan, "orphan = Orphan\n").expect("write orphan");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: false,
            dry_run: false,
            force_run: false,
            orphaned: true,
        });

        assert!(result.is_ok());
        assert!(
            !orphan.exists(),
            "explicit --orphaned should scan non-fallback locale files"
        );
    }

    #[test]
    fn run_clean_orphaned_all_still_runs_runner_with_all_locales() {
        let temp = crate::test_fixtures::create_workspace_with_locales(&[
            ("en", "hello = Hello\n"),
            ("es", "hello = Hola\n"),
        ]);
        let args_path = temp.path().join("runner-args.txt");
        crate::test_fixtures::setup_fake_runner_and_cache(
            &temp,
            FakeRunnerBehavior::record_args(&args_path),
        );
        let orphan = temp.path().join("i18n/es/orphan.ftl");
        fs::write(&orphan, "orphan = Orphan\n").expect("write orphan");

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: true,
            dry_run: false,
            force_run: false,
            orphaned: true,
        });

        assert!(result.is_ok());
        let args = fs::read_to_string(args_path).expect("read recorded runner args");
        assert!(
            args.contains(r#""command":"clean""#) && args.contains(r#""all_locales":true"#),
            "clean --orphaned --all-locales should run the normal all-locale clean request, got {args}"
        );
        assert!(
            !orphan.exists(),
            "clean --all-locales --orphaned should remove file orphans"
        );
    }

    #[test]
    fn run_clean_orphaned_all_requires_library_targets() {
        let temp = crate::test_fixtures::create_binary_only_i18n_workspace();

        let result = run_clean(CleanArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all_locales: true,
            dry_run: true,
            force_run: false,
            orphaned: true,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("library target"))
        );
        assert!(
            !temp.path().join(".es-fluent").exists(),
            "clean --all-locales --orphaned should fail before runner setup for binary-only crates"
        );
    }
}
