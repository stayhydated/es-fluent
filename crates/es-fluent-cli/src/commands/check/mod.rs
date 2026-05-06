//! Check command for validating FTL files against inventory-registered types.
//!
//! This module provides functionality to check FTL files by:
//! - Running a temp crate that collects inventory registrations (expected keys/variables)
//! - Parsing FTL files directly using fluent-syntax (for proper ParserError handling)
//! - Comparing FTL files against the expected keys and variables from Rust code
//! - Reporting missing keys as errors
//! - Reporting unexpected FTL variables as errors
//! - Reporting Rust-declared variables omitted by translations as warnings

mod inventory;
mod validation;

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, ValidationExecutionError, ValidationIssue, ValidationReport};
use crate::generation::MonolithicExecutor;
use crate::utils::ui;
use clap::Parser;
use miette::NamedSource;
use serde::Serialize;
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

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

pub(crate) struct CheckRun {
    pub(crate) crates_discovered: usize,
    pub(crate) crates_checked: usize,
    pub(crate) issues: Vec<ValidationIssue>,
}

#[derive(Serialize)]
struct CheckJsonReport {
    crates_discovered: usize,
    crates_checked: usize,
    error_count: usize,
    warning_count: usize,
    issues: Vec<CheckIssueJson>,
}

#[derive(Serialize)]
struct CheckIssueJson {
    severity: &'static str,
    kind: &'static str,
    source: String,
    locale: String,
    key: Option<String>,
    variable: Option<String>,
    help: String,
}

impl CheckJsonReport {
    fn from_run(run: &CheckRun) -> Self {
        let (error_count, warning_count) = count_issues(&run.issues);

        Self {
            crates_discovered: run.crates_discovered,
            crates_checked: run.crates_checked,
            error_count,
            warning_count,
            issues: run.issues.iter().map(CheckIssueJson::from).collect(),
        }
    }
}

impl From<&ValidationIssue> for CheckIssueJson {
    fn from(issue: &ValidationIssue) -> Self {
        match issue {
            ValidationIssue::MissingKey(error) => Self {
                severity: "error",
                kind: "missing_key",
                source: error.src.name().to_string(),
                locale: error.locale.clone(),
                key: Some(error.key.clone()),
                variable: None,
                help: error.help.clone(),
            },
            ValidationIssue::DuplicateKey(error) => Self {
                severity: "error",
                kind: "duplicate_key",
                source: error.src.name().to_string(),
                locale: error.locale.clone(),
                key: Some(error.key.clone()),
                variable: None,
                help: error.help.clone(),
            },
            ValidationIssue::MissingVariable(error) => Self {
                severity: "warning",
                kind: "missing_variable",
                source: error.src.name().to_string(),
                locale: error.locale.clone(),
                key: Some(error.key.clone()),
                variable: Some(error.variable.clone()),
                help: error.help.clone(),
            },
            ValidationIssue::UnexpectedVariable(error) => Self {
                severity: "error",
                kind: "unexpected_variable",
                source: error.src.name().to_string(),
                locale: error.locale.clone(),
                key: Some(error.key.clone()),
                variable: Some(error.variable.clone()),
                help: error.help.clone(),
            },
            ValidationIssue::ValidationExecution(error) => Self {
                severity: "error",
                kind: "validation_execution",
                source: error.src.name().to_string(),
                locale: String::new(),
                key: None,
                variable: None,
                help: error.help.clone(),
            },
            ValidationIssue::SyntaxError(error) => Self {
                severity: "error",
                kind: "syntax_error",
                source: error.src.name().to_string(),
                locale: error.locale.clone(),
                key: None,
                variable: None,
                help: error.help.clone(),
            },
        }
    }
}

pub(crate) fn count_issues(issues: &[ValidationIssue]) -> (usize, usize) {
    let error_count = issues
        .iter()
        .filter(|i| {
            matches!(
                i,
                ValidationIssue::MissingKey(_)
                    | ValidationIssue::DuplicateKey(_)
                    | ValidationIssue::UnexpectedVariable(_)
                    | ValidationIssue::ValidationExecution(_)
                    | ValidationIssue::SyntaxError(_)
            )
        })
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| matches!(i, ValidationIssue::MissingVariable(_)))
        .count();

    (error_count, warning_count)
}

pub(crate) fn collect_check_run(
    workspace: &WorkspaceCrates,
    all: bool,
    ignore: &[String],
    force_run: bool,
    show_progress: bool,
) -> Result<CheckRun, CliError> {
    // Convert ignore list to a HashSet for efficient lookups
    let ignore_crates: HashSet<String> = ignore.iter().cloned().collect();

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
        if show_progress {
            ui::Ui::print_no_crates_found();
        }
        return Ok(CheckRun {
            crates_discovered: workspace.crates.len(),
            crates_checked: 0,
            issues: Vec::new(),
        });
    }

    // Prepare monolithic temp crate once for all checks
    crate::generation::prepare_monolithic_runner_crate(&workspace.workspace_info)
        .map_err(|e| CliError::Other(e.to_string()))?;

    // First pass: collect all expected keys from crates
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(
        &workspace.workspace_info.root_dir,
    );
    let executor = MonolithicExecutor::new(&workspace.workspace_info);

    let pb = if show_progress {
        ui::Ui::create_progress_bar(crates_to_check.len() as u64, "Collecting keys...")
    } else {
        indicatif::ProgressBar::hidden()
    };

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

    let pb = if show_progress {
        ui::Ui::create_progress_bar(crates_to_check.len() as u64, "Checking crates...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    for krate in &crates_to_check {
        pb.set_message(format!("Checking {}", krate.name));

        match validation::validate_crate(
            krate,
            &workspace.workspace_info.root_dir,
            temp_store.base_dir(),
            all,
        ) {
            Ok(issues) => {
                all_issues.extend(issues);
            },
            Err(e) => {
                let error = e.to_string();
                // If error, print above progress bar
                if show_progress {
                    pb.suspend(|| {
                        ui::Ui::print_check_error(&krate.name, &error);
                    });
                }
                all_issues.push(ValidationIssue::ValidationExecution(
                    ValidationExecutionError {
                        src: NamedSource::new(krate.name.clone(), String::new()),
                        crate_name: krate.name.clone(),
                        help: error,
                    },
                ));
            },
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    // Sort issues for deterministic output
    all_issues.sort_by_cached_key(|issue| issue.sort_key());

    Ok(CheckRun {
        crates_discovered: workspace.crates.len(),
        crates_checked: crates_to_check.len(),
        issues: all_issues,
    })
}

/// Run the check command.
pub fn run_check(args: CheckArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;
    let show_text = !args.output.is_json();

    if show_text && !workspace.print_discovery(ui::Ui::print_check_header) {
        return Ok(());
    }

    let run = collect_check_run(
        &workspace,
        args.all,
        &args.ignore,
        args.force_run,
        show_text,
    )?;
    let (error_count, warning_count) = count_issues(&run.issues);

    if args.output.is_json() {
        args.output.print_json(&CheckJsonReport::from_run(&run))?;
        if !run.issues.is_empty() {
            return Err(CliError::Exit(1));
        }
        return Ok(());
    }

    if run.issues.is_empty() {
        ui::Ui::print_check_success();
        Ok(())
    } else {
        Err(CliError::Validation(ValidationReport {
            error_count,
            warning_count,
            issues: run.issues,
        }))
    }
}

#[cfg(test)]
mod tests;
