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
use crate::core::{
    CliError, OrphanedFtlFileError, ValidationExecutionError, ValidationIssue, ValidationReport,
};
use crate::generation::MonolithicExecutor;
use crate::utils::ui;
use clap::Parser;
use miette::NamedSource;
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// Arguments for the check command.
#[derive(bon::Builder, Debug, Parser)]
#[builder(on(String, into))]
pub struct CheckArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Include non-fallback validation, fallback-copy warnings, and orphan-file checks.
    #[arg(long)]
    pub all: bool,

    /// Crates to skip during validation. Can be specified multiple times
    /// (e.g., --ignore foo --ignore bar) or comma-separated (e.g., --ignore "foo, bar").
    #[arg(long, value_delimiter = ',')]
    pub ignore: Vec<String>,

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Disable --all warnings for non-fallback messages that match the fallback locale; requires --all.
    #[arg(long = "no-fallback-copy-check", action = clap::ArgAction::SetFalse, default_value_t = true)]
    #[builder(default = true)]
    pub check_fallback_copies: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

pub(crate) struct CheckRun {
    pub(crate) crates_discovered: usize,
    pub(crate) crates_checked: usize,
    pub(crate) workspace_warnings: Vec<String>,
    pub(crate) issues: Vec<ValidationIssue>,
}

#[derive(Serialize)]
struct CheckJsonReport {
    crates_discovered: usize,
    crates_checked: usize,
    workspace_warnings: Vec<String>,
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
    fn from_run(run: &CheckRun, workspace_root: &Path) -> Self {
        let (error_count, warning_count) = count_issues(&run.issues);

        Self {
            crates_discovered: run.crates_discovered,
            crates_checked: run.crates_checked,
            workspace_warnings: run.workspace_warnings.clone(),
            error_count,
            warning_count,
            issues: run
                .issues
                .iter()
                .map(|issue| CheckIssueJson::from_issue(issue, workspace_root))
                .collect(),
        }
    }

    fn command_error(crates_discovered: usize, error: impl ToString) -> Self {
        Self {
            crates_discovered,
            crates_checked: 0,
            workspace_warnings: Vec::new(),
            error_count: 1,
            warning_count: 0,
            issues: vec![CheckIssueJson {
                severity: "error",
                kind: "command_error",
                source: "workspace".to_string(),
                locale: String::new(),
                key: None,
                variable: None,
                help: error.to_string(),
            }],
        }
    }

    fn command_error_for_workspace(
        crates_discovered: usize,
        error: impl ToString,
        workspace_root: &Path,
    ) -> Self {
        let mut report = Self::command_error(crates_discovered, error);
        for issue in &mut report.issues {
            issue.help = relative_check_message(&issue.help, workspace_root);
        }
        report
    }
}

impl CheckIssueJson {
    fn from_issue(issue: &ValidationIssue, workspace_root: &Path) -> Self {
        let mut json = Self::from(issue);
        json.help = relative_check_message(&json.help, workspace_root);
        json
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
            ValidationIssue::UntranslatedMessage(error) => Self {
                severity: "warning",
                kind: "untranslated_message",
                source: error.src.name().to_string(),
                locale: error.locale.clone(),
                key: Some(error.key.clone()),
                variable: None,
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
            ValidationIssue::OrphanedFtlFile(error) => Self {
                severity: "error",
                kind: "orphaned_file",
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
                    | ValidationIssue::OrphanedFtlFile(_)
            )
        })
        .count();
    let warning_count = issues
        .iter()
        .filter(|i| {
            matches!(
                i,
                ValidationIssue::MissingVariable(_) | ValidationIssue::UntranslatedMessage(_)
            )
        })
        .count();

    (error_count, warning_count)
}

pub(crate) fn collect_check_run(
    workspace: &WorkspaceCrates,
    all: bool,
    ignore: &[String],
    force_run: bool,
    check_fallback_copies: bool,
    show_progress: bool,
) -> Result<CheckRun, CliError> {
    // Convert ignore list to a HashSet for efficient lookups
    let ignore_crates = normalize_ignore_crates(ignore)?;

    if workspace.crates.is_empty() {
        return Ok(CheckRun {
            crates_discovered: 0,
            crates_checked: 0,
            workspace_warnings: empty_check_set_warnings(workspace, &ignore_crates),
            issues: Vec::new(),
        });
    }

    // Filter out ignored crates
    let crates_to_check: Vec<_> = workspace
        .valid
        .iter()
        .filter(|k| !ignore_crates.contains(k.name.as_str()))
        .collect();
    let skipped_to_report: Vec<_> = workspace
        .skipped
        .iter()
        .filter(|k| !ignore_crates.contains(k.name.as_str()))
        .collect();

    validate_known_ignore_crates(workspace, &ignore_crates)?;

    let mut all_issues: Vec<ValidationIssue> = skipped_to_report
        .iter()
        .map(|krate| {
            ValidationIssue::ValidationExecution(ValidationExecutionError {
                src: NamedSource::new(&krate.name, String::new()),
                crate_name: krate.name.to_string(),
                help: "Crate has i18n.toml but no Cargo library target. Add src/lib.rs or a [lib] path in Cargo.toml.".to_string(),
            })
        })
        .collect();
    let (locale_setup_issues, locale_setup_issue_crates) =
        locale_setup_issues_for_crates(&crates_to_check);
    all_issues.extend(locale_setup_issues);
    let crates_ready_for_validation: Vec<_> = crates_to_check
        .iter()
        .copied()
        .filter(|krate| !locale_setup_issue_crates.contains(krate.name.as_str()))
        .collect();

    if crates_to_check.is_empty() {
        let workspace_warnings = empty_check_set_warnings(workspace, &ignore_crates);
        all_issues.sort_by_cached_key(|issue| issue.sort_key());
        return Ok(CheckRun {
            crates_discovered: workspace.crates.len(),
            crates_checked: 0,
            workspace_warnings,
            issues: all_issues,
        });
    }

    if crates_ready_for_validation.is_empty() {
        all_issues.sort_by_cached_key(|issue| issue.sort_key());
        return Ok(CheckRun {
            crates_discovered: workspace.crates.len(),
            crates_checked: 0,
            workspace_warnings: Vec::new(),
            issues: all_issues,
        });
    }

    let runner_workspace = crate::core::WorkspaceInfo {
        root_dir: workspace.workspace_info.root_dir.clone(),
        target_dir: workspace.workspace_info.target_dir.clone(),
        crates: crates_ready_for_validation
            .iter()
            .map(|krate| (*krate).clone())
            .collect(),
    };

    let _runner_lock =
        crate::generation::acquire_monolithic_runner_lock(&runner_workspace.root_dir)
            .map_err(|e| CliError::Other(e.to_string()))?;

    // Prepare monolithic temp crate once for all crates that can reach validation.
    crate::generation::prepare_monolithic_runner_crate(&runner_workspace)
        .map_err(|e| CliError::Other(e.to_string()))?;

    // First pass: collect all expected keys from crates
    let temp_store = es_fluent_runner::RunnerMetadataStore::temp_for_workspace(
        &workspace.workspace_info.root_dir,
    );
    let executor = MonolithicExecutor::new(&runner_workspace);

    let pb = if show_progress {
        ui::Ui::create_progress_bar(
            crates_ready_for_validation.len() as u64,
            "Collecting keys...",
        )
    } else {
        indicatif::ProgressBar::hidden()
    };

    for krate in &crates_ready_for_validation {
        pb.set_message(format!("Scanning {}", krate.name));
        let request = krate.check_request();
        executor
            .execute_request(&request, force_run)
            .map_err(|e| CliError::Other(e.to_string()))?;
        pb.inc(1);
    }

    pb.finish_and_clear();

    // Second pass: validate FTL files
    let pb = if show_progress {
        ui::Ui::create_progress_bar(
            crates_ready_for_validation.len() as u64,
            "Checking crates...",
        )
    } else {
        indicatif::ProgressBar::hidden()
    };

    for krate in &crates_ready_for_validation {
        pb.set_message(format!("Checking {}", krate.name));

        match validation::validate_crate(
            krate,
            &workspace.workspace_info.root_dir,
            temp_store.base_dir(),
            all,
            check_fallback_copies,
        ) {
            Ok(issues) => {
                all_issues.extend(issues);
            },
            Err(e) => {
                let error = e.to_string();
                // If error, print above progress bar
                if show_progress {
                    pb.suspend(|| {
                        ui::Ui::print_check_error(krate.name.as_str(), &error);
                    });
                }
                all_issues.push(ValidationIssue::ValidationExecution(
                    ValidationExecutionError {
                        src: NamedSource::new(&krate.name, String::new()),
                        crate_name: krate.name.to_string(),
                        help: error,
                    },
                ));
            },
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    let filtered_crates: Vec<_> = crates_to_check
        .iter()
        .filter(|krate| !locale_setup_issue_crates.contains(krate.name.as_str()))
        .map(|krate| (*krate).clone())
        .collect();
    for orphaned in super::clean::orphaned::find_orphaned_file_infos_for_workspace(
        workspace,
        &filtered_crates,
        all,
    )? {
        let source = relative_path(&orphaned.abs_path, &workspace.workspace_info.root_dir);
        all_issues.push(ValidationIssue::OrphanedFtlFile(OrphanedFtlFileError {
            src: NamedSource::new(source.clone(), String::new()),
            locale: orphaned.locale,
            path: source,
            help: "Remove this file or run `cargo es-fluent clean --orphaned`.".to_string(),
        }));
    }

    // Sort issues for deterministic output
    all_issues.sort_by_cached_key(|issue| issue.sort_key());

    Ok(CheckRun {
        crates_discovered: workspace.crates.len(),
        crates_checked: crates_ready_for_validation.len(),
        workspace_warnings: Vec::new(),
        issues: all_issues,
    })
}

fn locale_setup_issues_for_crates(
    crates: &[&crate::core::CrateInfo],
) -> (Vec<ValidationIssue>, HashSet<String>) {
    let mut issues = Vec::new();
    let mut issue_crates = HashSet::new();

    for krate in crates {
        let ctx = match crate::ftl::LocaleContext::from_crate(krate, false) {
            Ok(ctx) => ctx,
            Err(error) => {
                issue_crates.insert(krate.name.to_string());
                issues.push(ValidationIssue::ValidationExecution(
                    ValidationExecutionError {
                        src: NamedSource::new(&krate.name, String::new()),
                        crate_name: krate.name.to_string(),
                        help: error.to_string(),
                    },
                ));
                continue;
            },
        };

        if !ctx.assets_dir.is_dir() {
            issue_crates.insert(krate.name.to_string());
            issues.push(ValidationIssue::ValidationExecution(
                ValidationExecutionError {
                    src: NamedSource::new(&krate.name, String::new()),
                    crate_name: krate.name.to_string(),
                    help: format!(
                        "assets_dir for {} is missing or not a directory: {}",
                        krate.name,
                        ctx.assets_dir.display()
                    ),
                },
            ));
            continue;
        }

        let fallback_dir = ctx.locale_dir(&ctx.fallback);
        let fallback_path_invalid = !crate::ftl::is_real_locale_directory(&fallback_dir);
        if fallback_path_invalid {
            issue_crates.insert(krate.name.to_string());
            issues.push(ValidationIssue::ValidationExecution(
                ValidationExecutionError {
                    src: NamedSource::new(&krate.name, String::new()),
                    crate_name: krate.name.to_string(),
                    help: format!(
                        "fallback locale directory '{}' for {} is missing or not a directory: {}",
                        ctx.fallback,
                        krate.name,
                        fallback_dir.display()
                    ),
                },
            ));
            continue;
        }

        match crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir) {
            Ok(locale_path_issues) => {
                let locale_path_issues: Vec<_> = locale_path_issues
                    .into_iter()
                    .filter(|issue| !(fallback_path_invalid && issue.locale == ctx.fallback))
                    .collect();
                if !locale_path_issues.is_empty() {
                    issue_crates.insert(krate.name.to_string());
                }
                issues.extend(locale_path_issues.into_iter().map(|issue| {
                    ValidationIssue::ValidationExecution(ValidationExecutionError {
                        src: NamedSource::new(&krate.name, String::new()),
                        crate_name: krate.name.to_string(),
                        help: format!(
                            "Locale path '{}' is not a directory. Remove the file or replace it with a directory: {}",
                            issue.locale,
                            issue.path.display()
                        ),
                    })
                }));
            },
            Err(error) => {
                issue_crates.insert(krate.name.to_string());
                issues.push(ValidationIssue::ValidationExecution(
                    ValidationExecutionError {
                        src: NamedSource::new(&krate.name, String::new()),
                        crate_name: krate.name.to_string(),
                        help: error.to_string(),
                    },
                ));
            },
        }

        if let Err(error) = crate::ftl::LocaleContext::from_crate(krate, true) {
            issue_crates.insert(krate.name.to_string());
            issues.push(ValidationIssue::ValidationExecution(
                ValidationExecutionError {
                    src: NamedSource::new(&krate.name, String::new()),
                    crate_name: krate.name.to_string(),
                    help: error.to_string(),
                },
            ));
        }

        if let Ok(all_ctx) = crate::ftl::LocaleContext::from_crate(krate, true) {
            for locale in &all_ctx.locales {
                let locale_dir = all_ctx.locale_dir(locale);
                if !crate::ftl::is_real_locale_directory(&locale_dir) {
                    continue;
                }

                if let Err(error) = crate::ftl::CrateFtlLayout::from_assets_dir(
                    &all_ctx.assets_dir,
                    locale,
                    &all_ctx.crate_name,
                )
                .discover_files()
                {
                    issue_crates.insert(krate.name.to_string());
                    issues.push(ValidationIssue::ValidationExecution(
                        ValidationExecutionError {
                            src: NamedSource::new(&krate.name, String::new()),
                            crate_name: krate.name.to_string(),
                            help: format!("FTL file layout could not be read: {error}"),
                        },
                    ));
                }
            }
        }
    }

    (issues, issue_crates)
}

fn empty_check_set_warnings(
    workspace: &WorkspaceCrates,
    ignore_crates: &HashSet<String>,
) -> Vec<String> {
    if let Some(message) = workspace.empty_selection_message() {
        return vec![message];
    }

    if !ignore_crates.is_empty() {
        return vec!["all selected crates were ignored by --ignore".to_string()];
    }

    Vec::new()
}

fn normalize_ignore_crates(ignore: &[String]) -> Result<HashSet<String>, CliError> {
    let mut normalized = HashSet::new();
    for krate in ignore {
        let krate = krate.trim();
        if krate.is_empty() {
            return Err(CliError::Other(
                "ignore values must not be empty; remove empty entries from comma-separated lists"
                    .to_string(),
            ));
        }
        normalized.insert(krate.to_string());
    }
    Ok(normalized)
}

fn validate_known_ignore_crates(
    workspace: &WorkspaceCrates,
    ignore_crates: &HashSet<String>,
) -> Result<(), CliError> {
    if ignore_crates.is_empty() {
        return Ok(());
    }

    let all_crate_names: HashSet<String> =
        workspace.all_i18n_package_names.iter().cloned().collect();

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

    Ok(())
}

/// Run the check command.
pub fn run_check(args: CheckArgs) -> Result<(), CliError> {
    let output = args.output;
    if !args.all && !args.check_fallback_copies {
        let error = CliError::Other(
            "--no-fallback-copy-check requires --all because fallback-copy warnings only run during all-locale checks"
                .to_string(),
        );
        if output.is_json() {
            output.print_json(&CheckJsonReport::command_error(0, error))?;
            return Err(CliError::Exit(1));
        }
        return Err(error);
    }

    let ignore_crates = match normalize_ignore_crates(&args.ignore) {
        Ok(ignore_crates) => ignore_crates,
        Err(error) if output.is_json() => {
            output.print_json(&CheckJsonReport::command_error(0, error))?;
            return Err(CliError::Exit(1));
        },
        Err(error) => return Err(error),
    };
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) if output.is_json() => {
            output.print_json(&CheckJsonReport {
                crates_discovered: 0,
                crates_checked: 0,
                workspace_warnings: Vec::new(),
                error_count: 1,
                warning_count: 0,
                issues: vec![CheckIssueJson {
                    severity: "error",
                    kind: "setup_error",
                    source: "workspace".to_string(),
                    locale: String::new(),
                    key: None,
                    variable: None,
                    help: error.to_string(),
                }],
            })?;
            return Err(CliError::Exit(1));
        },
        Err(error) => return Err(error),
    };
    let show_text = !output.is_json();

    if show_text && !workspace.crates.is_empty() {
        validate_known_ignore_crates(&workspace, &ignore_crates)?;
    }

    if show_text {
        workspace.print_discovery(ui::Ui::print_check_header);
    }

    let run = match collect_check_run(
        &workspace,
        args.all,
        &args.ignore,
        args.force_run,
        args.check_fallback_copies,
        show_text,
    ) {
        Ok(run) => run,
        Err(error) if output.is_json() => {
            output.print_json(&CheckJsonReport::command_error_for_workspace(
                workspace.crates.len(),
                error,
                &workspace.workspace_info.root_dir,
            ))?;
            return Err(CliError::Exit(1));
        },
        Err(error) => return Err(error),
    };
    let (error_count, warning_count) = count_issues(&run.issues);

    if output.is_json() {
        output.print_json(&CheckJsonReport::from_run(
            &run,
            &workspace.workspace_info.root_dir,
        ))?;
        if !run.issues.is_empty() {
            return Err(CliError::Exit(1));
        }
        return Ok(());
    }

    for warning in &run.workspace_warnings {
        println!("workspace warning: {warning}");
    }

    if run.issues.is_empty() {
        if run.workspace_warnings.is_empty() {
            ui::Ui::print_check_success();
        }
        Ok(())
    } else {
        Err(CliError::Validation(ValidationReport {
            error_count,
            warning_count,
            issues: run.issues,
        }))
    }
}

fn relative_path(path: &Path, base: &Path) -> String {
    crate::utils::paths::relative_slash_path(path, base)
}

fn relative_check_message(message: &str, base: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, base)
}

#[cfg(test)]
mod tests;
