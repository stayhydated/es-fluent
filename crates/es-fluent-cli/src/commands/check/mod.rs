//! Check command for validating FTL files against inventory-registered types.
//!
//! This module provides functionality to check FTL files by:
//! - Running a temp crate that collects inventory registrations (expected keys/variables)
//! - Parsing FTL files directly using fluent-syntax (for proper ParserError handling)
//! - Comparing FTL files against the expected keys and variables from Rust code
//! - Reporting missing keys as errors
//! - Reporting missing variables as warnings

mod inventory;
mod validation;

use super::common::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, ValidationIssue, ValidationReport};
use crate::generation::{MonolithicExecutor, prepare_monolithic_runner_crate};
use crate::utils::ui;
use clap::Parser;
use std::collections::HashSet;

/// Arguments for the check command.
#[derive(Debug, Parser)]
pub struct CheckArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Check all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Crates to skip during validation. Can be specified multiple times
    /// (e.g., --ignore foo --ignore bar) or comma-separated (e.g., --ignore foo,bar).
    #[arg(long, value_delimiter = ',')]
    pub ignore: Vec<String>,

    /// Force rebuild of the runner, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,
}

/// Run the check command.
pub fn run_check(args: CheckArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::Ui::print_check_header) {
        ui::Ui::print_no_crates_found();
        return Ok(());
    }

    // Convert ignore list to a HashSet for efficient lookups
    let ignore_crates: HashSet<String> = args.ignore.into_iter().collect();
    let force_run = args.force_run;

    // Filter out ignored crates
    let crates_to_check: Vec<_> = workspace
        .valid
        .iter()
        .filter(|k| !ignore_crates.contains(&k.name))
        .collect();

    // Validate that all ignored crates are known
    if !ignore_crates.is_empty() {
        let all_crate_names: HashSet<String> =
            workspace.valid.iter().map(|k| k.name.clone()).collect();

        let mut unknown_crates: Vec<&String> = ignore_crates
            .iter()
            .filter(|c| !all_crate_names.contains(*c))
            .collect();

        if !unknown_crates.is_empty() {
            // Sort for deterministic error messages
            unknown_crates.sort();

            return Err(CliError::Other(format!(
                "Unknown crates passed to --ignore: {}",
                unknown_crates
                    .iter()
                    .map(|c| format!("'{}'", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    if crates_to_check.is_empty() {
        ui::Ui::print_no_crates_found();
        return Ok(());
    }

    // Prepare monolithic temp crate once for all checks
    prepare_monolithic_runner_crate(&workspace.workspace_info)
        .map_err(|e| CliError::Other(e.to_string()))?;

    // First pass: collect all expected keys from crates
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(
        &workspace.workspace_info.root_dir,
    );
    let executor = MonolithicExecutor::new(&workspace.workspace_info);

    let pb = ui::Ui::create_progress_bar(crates_to_check.len() as u64, "Collecting keys...");

    for krate in &crates_to_check {
        pb.set_message(format!("Scanning {}", krate.name));
        let request = krate.check_request();
        executor
            .execute_request(&request, force_run)
            .map_err(|e| CliError::Other(e.to_string()))?;
        pb.inc(1);
    }

    pb.finish_and_clear();

    // Second pass: validate FTL files
    let mut all_issues: Vec<ValidationIssue> = Vec::new();

    let pb = ui::Ui::create_progress_bar(crates_to_check.len() as u64, "Checking crates...");

    for krate in &crates_to_check {
        pb.set_message(format!("Checking {}", krate.name));

        match validation::validate_crate(
            krate,
            &workspace.workspace_info.root_dir,
            temp_store.base_dir(),
            args.all,
        ) {
            Ok(issues) => {
                all_issues.extend(issues);
            },
            Err(e) => {
                // If error, print above progress bar
                pb.suspend(|| {
                    ui::Ui::print_check_error(&krate.name, &e.to_string());
                });
            },
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    // Sort issues for deterministic output
    all_issues.sort_by_cached_key(|issue| issue.sort_key());

    let error_count = all_issues
        .iter()
        .filter(|i| {
            matches!(
                i,
                ValidationIssue::MissingKey(_) | ValidationIssue::SyntaxError(_)
            )
        })
        .count();
    let warning_count = all_issues
        .iter()
        .filter(|i| matches!(i, ValidationIssue::MissingVariable(_)))
        .count();

    if all_issues.is_empty() {
        ui::Ui::print_check_success();
        Ok(())
    } else {
        Err(CliError::Validation(ValidationReport {
            error_count,
            warning_count,
            issues: all_issues,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::test_fixtures::{
        INVENTORY_WITH_HELLO, INVENTORY_WITH_MISSING_KEY, RUNNER_FAILING_SCRIPT, RUNNER_SCRIPT,
        create_test_crate_workspace, setup_fake_runner_and_cache as setup_runner_cache,
    };

    fn setup_fake_runner_and_cache_with_script(temp: &tempfile::TempDir, script: &str) {
        setup_runner_cache(temp, script);
    }

    fn setup_fake_runner_and_cache(temp: &tempfile::TempDir) {
        setup_fake_runner_and_cache_with_script(temp, RUNNER_SCRIPT);
    }

    #[test]
    fn run_check_returns_error_for_unknown_ignored_crate() {
        let temp = create_test_crate_workspace();

        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            ignore: vec!["missing-crate".to_string()],
            force_run: false,
        });

        assert!(
            matches!(result, Err(CliError::Other(msg)) if msg.contains("Unknown crates passed to --ignore"))
        );
    }

    #[test]
    fn run_check_returns_ok_when_package_filter_matches_nothing() {
        let temp = create_test_crate_workspace();

        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-crate".to_string()),
            },
            all: false,
            ignore: Vec::new(),
            force_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_check_succeeds_with_fake_runner_and_matching_inventory() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache(&temp);

        let inventory_path =
            es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
                .inventory_path("test-app");
        fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
        fs::write(&inventory_path, INVENTORY_WITH_HELLO).expect("write inventory");

        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            ignore: Vec::new(),
            force_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_check_returns_validation_error_for_missing_key() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache(&temp);

        let inventory_path =
            es_fluent_runner::RunnerMetadataStore::new(temp.path().join(".es-fluent"))
                .inventory_path("test-app");
        fs::create_dir_all(inventory_path.parent().unwrap()).expect("create inventory dir");
        fs::write(&inventory_path, INVENTORY_WITH_MISSING_KEY).expect("write inventory");

        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            ignore: Vec::new(),
            force_run: false,
        });

        assert!(matches!(result, Err(CliError::Validation(_))));
    }

    #[test]
    fn run_check_returns_ok_when_all_crates_are_ignored() {
        let temp = create_test_crate_workspace();
        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            ignore: vec!["test-app".to_string()],
            force_run: false,
        });

        assert!(result.is_ok());
    }

    #[test]
    fn run_check_returns_other_error_when_runner_execution_fails() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache_with_script(&temp, RUNNER_FAILING_SCRIPT);

        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            ignore: Vec::new(),
            force_run: false,
        });

        assert!(matches!(result, Err(CliError::Other(_))));
    }

    #[test]
    fn run_check_handles_validation_errors_per_crate_and_completes() {
        let temp = create_test_crate_workspace();
        setup_fake_runner_and_cache(&temp);
        // Intentionally do not create inventory file so validation::validate_crate fails.

        let result = run_check(CheckArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            ignore: Vec::new(),
            force_run: false,
        });

        assert!(
            result.is_ok(),
            "per-crate validation errors should be reported and command should complete"
        );
    }
}
