//! Format command for sorting FTL entries alphabetically (A-Z).
//!
//! This module provides functionality to format FTL files by sorting
//! message keys alphabetically while preserving group comments.

use crate::commands::{DryRunDiff, WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo, FormatError, FormatReport};
use crate::ftl::LocaleContext;
use crate::utils::{discover_ftl_files, ui};
use anyhow::Result;
use clap::Parser;
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
    /// Diff info (original, new) if dry run and changed.
    pub diff_info: Option<DryRunDiff>,
}

impl FormatResult {
    /// Create an error result.
    fn error(path: &Path, msg: impl Into<String>) -> Self {
        Self {
            path: path.to_path_buf(),
            changed: false,
            error: Some(msg.into()),
            diff_info: None,
        }
    }

    /// Create an unchanged result.
    fn unchanged(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            changed: false,
            error: None,
            diff_info: None,
        }
    }

    /// Create a changed result with optional diff info.
    fn changed(path: &Path, diff: Option<DryRunDiff>) -> Self {
        Self {
            path: path.to_path_buf(),
            changed: true,
            error: None,
            diff_info: diff,
        }
    }
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
                    let display_path = std::env::current_dir()
                        .ok()
                        .and_then(|cwd| result.path.strip_prefix(&cwd).ok())
                        .unwrap_or(&result.path);

                    if args.dry_run {
                        ui::print_would_format(display_path);
                        if let Some(diff) = &result.diff_info {
                            diff.print();
                        }
                    } else {
                        ui::print_formatted(display_path);
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
    let ctx = LocaleContext::from_crate(krate, all_locales)?;

    let mut results = Vec::new();

    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        if !locale_dir.exists() {
            continue;
        }

        // Format main + namespaced files for this crate.
        let ftl_files = discover_ftl_files(&ctx.assets_dir, locale, &ctx.crate_name)?;
        for file_info in ftl_files {
            let ftl_file = fs::canonicalize(&file_info.abs_path).unwrap_or(file_info.abs_path);
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
        Err(e) => return FormatResult::error(path, format!("Failed to read file: {}", e)),
    };

    if content.trim().is_empty() {
        return FormatResult::unchanged(path);
    }

    let resource = match parser::parse(content.clone()) {
        Ok(res) => res,
        Err((res, _errors)) => res, // Use the partial result even with errors
    };

    // Use shared formatting logic from es-fluent-generate
    let formatted = es_fluent_generate::formatting::sort_ftl_resource(&resource);
    let formatted_content = format!("{}\n", formatted.trim_end());

    if content == formatted_content {
        return FormatResult::unchanged(path);
    }

    // Try to write if not in check-only mode
    if !check_only && let Err(e) = fs::write(path, &formatted_content) {
        return FormatResult::error(path, format!("Failed to write file: {}", e));
    }

    let diff = if check_only {
        Some(DryRunDiff::new(content, formatted_content))
    } else {
        None
    };

    FormatResult::changed(path, diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_test_crate(temp_dir: &Path) -> CrateInfo {
        let src_dir = temp_dir.join("src");
        let assets_dir = temp_dir.join("i18n/en");
        std::fs::create_dir_all(&src_dir).expect("create src");
        std::fs::create_dir_all(&assets_dir).expect("create assets");
        std::fs::create_dir_all(assets_dir.join("test-crate")).expect("create namespace dir");

        let config_path = temp_dir.join("i18n.toml");
        std::fs::write(
            &config_path,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        // Main file unchanged.
        std::fs::write(assets_dir.join("test-crate.ftl"), "hello = Hello\n")
            .expect("write main ftl");

        // Namespaced file intentionally unsorted.
        std::fs::write(
            assets_dir.join("test-crate/ui.ftl"),
            "zeta = Z\nalpha = A\n",
        )
        .expect("write namespaced ftl");

        CrateInfo {
            name: "test-crate".to_string(),
            manifest_dir: temp_dir.to_path_buf(),
            src_dir,
            i18n_config_path: config_path,
            ftl_output_dir: temp_dir.join("i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    #[test]
    fn format_crate_formats_namespaced_files() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let krate = write_test_crate(temp.path());

        let results = format_crate(&krate, false, false).expect("format crate");
        assert_eq!(
            results.len(),
            2,
            "main + namespaced files should be visited"
        );

        let namespaced_path = temp.path().join("i18n/en/test-crate/ui.ftl");
        let namespaced_suffix = Path::new("test-crate").join("ui.ftl");
        let namespaced_result = results
            .iter()
            .find(|r| r.path.ends_with(&namespaced_suffix))
            .expect("namespaced result exists");
        assert!(
            namespaced_result.changed,
            "namespaced file should be formatted"
        );

        let content = std::fs::read_to_string(&namespaced_path).expect("read namespaced file");
        assert!(
            content.starts_with("alpha = A\nzeta = Z"),
            "expected sorted content, got:\n{content}"
        );
    }

    #[test]
    fn format_crate_dry_run_keeps_namespaced_file_unchanged() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let krate = write_test_crate(temp.path());
        let namespaced_path = temp.path().join("i18n/en/test-crate/ui.ftl");
        let before = std::fs::read_to_string(&namespaced_path).expect("read before");

        let results = format_crate(&krate, false, true).expect("dry run format");
        let namespaced_suffix = Path::new("test-crate").join("ui.ftl");
        let namespaced_result = results
            .iter()
            .find(|r| r.path.ends_with(&namespaced_suffix))
            .expect("namespaced result exists");

        assert!(namespaced_result.changed);
        assert!(namespaced_result.diff_info.is_some());

        let after = std::fs::read_to_string(&namespaced_path).expect("read after");
        assert_eq!(before, after, "dry run should not write files");
    }
}
