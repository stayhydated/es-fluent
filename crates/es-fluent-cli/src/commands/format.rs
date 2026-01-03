//! Format command for sorting FTL entries alphabetically (A-Z).
//!
//! This module provides functionality to format FTL files by sorting
//! message keys alphabetically while preserving group comments.

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo, FormatError, FormatReport};
use crate::utils::{get_all_locales, ui};
use anyhow::{Context as _, Result};
use clap::Parser;
use es_fluent_toml::I18nConfig;
use fluent_syntax::parser;
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for the format command.
#[derive(Debug, Parser)]
pub struct FormatArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Format all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would be formatted without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

/// Result of formatting a single file.
#[derive(Debug)]
pub struct FormatResult {
    /// Path to the file.
    pub path: PathBuf,
    /// Whether the file was changed.
    pub changed: bool,
    /// Error if formatting failed.
    pub error: Option<String>,
}

/// Run the format command.
pub fn run_format(args: FormatArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_format_header) {
        ui::print_no_crates_found();
        return Ok(());
    }

    let mut total_formatted = 0;
    let mut total_unchanged = 0;
    let mut errors: Vec<FormatError> = Vec::new();

    let pb = ui::create_progress_bar(workspace.crates.len() as u64, "Formatting crates...");

    for krate in &workspace.crates {
        pb.set_message(format!("Formatting {}", krate.name));
        let results = format_crate(krate, args.all, args.dry_run)?;

        for result in results {
            if let Some(error) = result.error {
                errors.push(FormatError {
                    path: result.path,
                    help: error,
                });
            } else if result.changed {
                total_formatted += 1;
                pb.suspend(|| {
                    if args.dry_run {
                        ui::print_would_format(&result.path);
                    } else {
                        ui::print_formatted(&result.path);
                    }
                });
            } else {
                total_unchanged += 1;
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    if errors.is_empty() {
        if args.dry_run && total_formatted > 0 {
            ui::print_format_dry_run_summary(total_formatted);
        } else {
            ui::print_format_summary(total_formatted, total_unchanged);
        }
        Ok(())
    } else {
        Err(CliError::Format(FormatReport {
            formatted_count: total_formatted,
            error_count: errors.len(),
            errors,
        }))
    }
}

/// Format all FTL files for a crate.
fn format_crate(
    krate: &CrateInfo,
    all_locales: bool,
    check_only: bool,
) -> Result<Vec<FormatResult>> {
    let config = I18nConfig::read_from_path(&krate.i18n_config_path)
        .with_context(|| format!("Failed to read {}", krate.i18n_config_path.display()))?;

    let assets_dir = krate.manifest_dir.join(&config.assets_dir);

    let locales: Vec<String> = if all_locales {
        // Get all locale directories
        get_all_locales(&assets_dir)?
    } else {
        vec![config.fallback_language.clone()]
    };

    let mut results = Vec::new();

    for locale in &locales {
        let locale_dir = assets_dir.join(locale);
        if !locale_dir.exists() {
            continue;
        }

        // Format only the FTL file for this crate
        let ftl_file = locale_dir.join(format!("{}.ftl", krate.name));
        if ftl_file.exists() {
            let result = format_ftl_file(&ftl_file, check_only);
            results.push(result);
        }
    }

    Ok(results)
}

/// Format a single FTL file by sorting entries A-Z.
fn format_ftl_file(path: &Path, check_only: bool) -> FormatResult {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return FormatResult {
                path: path.to_path_buf(),
                changed: false,
                error: Some(format!("Failed to read file: {}", e)),
            };
        },
    };

    if content.trim().is_empty() {
        return FormatResult {
            path: path.to_path_buf(),
            changed: false,
            error: None,
        };
    }

    let resource = match parser::parse(content.clone()) {
        Ok(res) => res,
        Err((res, _errors)) => {
            // Use the partial result even with errors
            res
        },
    };

    // Use shared formatting logic from es-fluent-generate
    let formatted = es_fluent_generate::formatting::sort_ftl_resource(&resource);
    let formatted_content = format!("{}\n", formatted.trim_end());

    let changed = content != formatted_content;

    if changed
        && !check_only
        && let Err(e) = fs::write(path, &formatted_content)
    {
        return FormatResult {
            path: path.to_path_buf(),
            changed: false,
            error: Some(format!("Failed to write file: {}", e)),
        };
    }

    FormatResult {
        path: path.to_path_buf(),
        changed,
        error: None,
    }
}
