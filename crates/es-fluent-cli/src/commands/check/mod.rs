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

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, ValidationIssue, ValidationReport};
use crate::generation::{prepare_monolithic_runner_crate, run_monolithic};
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

    if !workspace.print_discovery(ui::print_check_header) {
        ui::print_no_crates_found();
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
        ui::print_no_crates_found();
        return Ok(());
    }

    // Prepare monolithic temp crate once for all checks
    prepare_monolithic_runner_crate(&workspace.workspace_info)
        .map_err(|e| CliError::Other(e.to_string()))?;

    // First pass: collect all expected keys from crates
    let temp_dir =
        es_fluent_derive_core::get_es_fluent_temp_dir(&workspace.workspace_info.root_dir);

    let pb = ui::create_progress_bar(crates_to_check.len() as u64, "Collecting keys...");

    for krate in &crates_to_check {
        pb.set_message(format!("Scanning {}", krate.name));
        run_monolithic(
            &workspace.workspace_info,
            "check",
            &krate.name,
            &[],
            force_run,
        )
        .map_err(|e| CliError::Other(e.to_string()))?;
        pb.inc(1);
    }

    pb.finish_and_clear();

    // Second pass: validate FTL files
    let mut all_issues: Vec<ValidationIssue> = Vec::new();

    let pb = ui::create_progress_bar(crates_to_check.len() as u64, "Checking crates...");

    for krate in &crates_to_check {
        pb.set_message(format!("Checking {}", krate.name));

        match validation::validate_crate(
            krate,
            &workspace.workspace_info.root_dir,
            &temp_dir,
            args.all,
        ) {
            Ok(issues) => {
                all_issues.extend(issues);
            },
            Err(e) => {
                // If error, print above progress bar
                pb.suspend(|| {
                    ui::print_check_error(&krate.name, &e.to_string());
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
        ui::print_check_success();
        Ok(())
    } else {
        Err(CliError::Validation(ValidationReport {
            error_count,
            warning_count,
            issues: all_issues,
        }))
    }
}
