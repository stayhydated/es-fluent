//! Status command implementation.

mod collectors;
mod report;

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use anstream::println;
use clap::Parser;

use collectors::{
    collect_format_status_results, collect_orphaned_status_paths, collect_status_generation_errors,
    collect_status_setup_errors, count_generation_stale_crates, count_status_validation_issues,
    normalize_status_setup_errors,
};
use report::{StatusReport, print_status_report};

/// Arguments for the status command.
#[derive(Debug, Parser)]
pub struct StatusArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Include non-fallback formatting, sync, orphan-file, and validation checks.
    #[arg(long)]
    pub all_locales: bool,

    /// Run the generated runner through Cargo, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
}

/// Run the status command.
pub fn run_status(args: StatusArgs) -> Result<(), CliError> {
    let output = args.output;
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) if output.is_json() => {
            let report = StatusReport {
                crates_discovered: 0,
                crates_checked: 0,
                workspace_warnings: Vec::new(),
                setup_errors: vec![error.to_string()],
                generation_stale_crates: 0,
                generation_errors: Vec::new(),
                cleanup_stale_crates: 0,
                cleanup_errors: Vec::new(),
                files_need_formatting: 0,
                format_errors: Vec::new(),
                missing_synced_keys: 0,
                locales_need_sync: 0,
                orphaned_files: Vec::new(),
                validation_errors: 0,
                validation_warnings: 0,
                clean: false,
            };
            output.print_json(&report)?;
            return Err(CliError::Exit(1));
        },
        Err(error) => return Err(error),
    };
    let show_text = !output.is_json();

    if show_text {
        println!("Fluent FTL Status");
    }

    let workspace_warnings: Vec<String> = workspace.empty_selection_message().into_iter().collect();
    let mut setup_errors = collect_status_setup_errors(&workspace);
    let skip_dependent_checks = !setup_errors.is_empty() || !workspace_warnings.is_empty();

    let generation_results = if skip_dependent_checks {
        Vec::new()
    } else {
        super::common::run_generation_for_crates(
            &workspace.workspace_info,
            &workspace.valid,
            &GenerationAction::Generate {
                mode: FluentParseMode::default(),
                dry_run: true,
            },
            args.force_run,
            show_text,
        )
    };
    let generation_stale_crates = count_generation_stale_crates(&generation_results);
    let generation_errors =
        collect_status_generation_errors(&generation_results, &workspace.workspace_info.root_dir);
    let cleanup_results = if skip_dependent_checks {
        Vec::new()
    } else {
        super::common::run_generation_for_crates(
            &workspace.workspace_info,
            &workspace.valid,
            &GenerationAction::Clean {
                all_locales: args.all_locales,
                dry_run: true,
            },
            args.force_run,
            show_text,
        )
    };
    let cleanup_stale_crates = count_generation_stale_crates(&cleanup_results);
    let cleanup_errors =
        collect_status_generation_errors(&cleanup_results, &workspace.workspace_info.root_dir);

    let mut files_need_formatting = 0;
    let mut format_errors = Vec::new();
    if !skip_dependent_checks {
        let format_results = collect_format_status_results(&workspace, args.all_locales);
        files_need_formatting = format_results.0;
        format_errors = format_results.1;
    }

    let mut missing_synced_keys = 0;
    let mut locales_need_sync = std::collections::HashSet::new();
    if args.all_locales && !skip_dependent_checks {
        for krate in &workspace.crates {
            match super::sync::sync_crate(krate, None, true, false) {
                Ok(results) => {
                    for result in results {
                        if result.keys_added > 0 {
                            missing_synced_keys += result.keys_added;
                            locales_need_sync
                                .insert((krate.name.to_string(), result.locale.clone()));
                        }
                    }
                },
                Err(error) => {
                    setup_errors.push(format!("{}: {}", krate.name, error));
                },
            }
        }
    }

    let orphaned_files = if skip_dependent_checks {
        Vec::new()
    } else {
        match collect_orphaned_status_paths(&workspace, args.all_locales) {
            Ok(files) => files,
            Err(error) => {
                setup_errors.push(error.to_string());
                Vec::new()
            },
        }
    };

    let (crates_checked, validation_errors, validation_warnings) = if skip_dependent_checks {
        (0, 0, 0)
    } else {
        let check_run = super::check::collect_check_run(
            &workspace,
            args.all_locales,
            &[],
            args.force_run,
            true,
            false,
        );
        match check_run {
            Ok(check_run) => {
                let (validation_errors, validation_warnings) =
                    count_status_validation_issues(&check_run.issues);
                (
                    check_run.crates_checked,
                    validation_errors,
                    validation_warnings,
                )
            },
            Err(error) => {
                setup_errors.push(error.to_string());
                (0, 1, 0)
            },
        }
    };

    setup_errors = normalize_status_setup_errors(setup_errors, &workspace.workspace_info.root_dir);
    setup_errors.sort();
    setup_errors.dedup();

    let clean = !workspace.crates.is_empty()
        && generation_stale_crates == 0
        && cleanup_stale_crates == 0
        && setup_errors.is_empty()
        && generation_errors.is_empty()
        && cleanup_errors.is_empty()
        && files_need_formatting == 0
        && format_errors.is_empty()
        && missing_synced_keys == 0
        && orphaned_files.is_empty()
        && validation_errors == 0
        && validation_warnings == 0;

    let report = StatusReport {
        crates_discovered: workspace.crates.len(),
        crates_checked,
        workspace_warnings,
        setup_errors,
        generation_stale_crates,
        generation_errors,
        cleanup_stale_crates,
        cleanup_errors,
        files_need_formatting,
        format_errors,
        missing_synced_keys,
        locales_need_sync: locales_need_sync.len(),
        orphaned_files,
        validation_errors,
        validation_warnings,
        clean,
    };

    if output.is_json() {
        output.print_json(&report)?;
    } else {
        print_status_report(&report);
    }

    if report.clean {
        Ok(())
    } else {
        Err(CliError::Exit(1))
    }
}

#[cfg(test)]
mod tests;
