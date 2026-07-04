//! Format command for sorting FTL entries alphabetically (A-Z).
//!
//! This module provides functionality to format FTL files by sorting
//! message keys alphabetically while preserving group comments.

use super::common::{OutputFormat, WorkspaceArgs, WorkspaceCrates};
use super::dry_run::{DryRunDiff, DryRunSummary};
use crate::core::{CliError, CrateInfo, FormatError, FormatReport};
use crate::ftl::{CrateFtlLayout, LocaleContext};
use crate::utils::ui;
use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for the format command.
#[derive(Debug, Parser)]
pub struct FormatArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Format all discovered locale directories, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would be formatted without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::default())]
    pub output: OutputFormat,
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

#[derive(Serialize)]
struct FormatJsonReport {
    dry_run: bool,
    formatted_count: usize,
    unchanged_count: usize,
    error_count: usize,
    files: Vec<FormatFileJson>,
    errors: Vec<String>,
}

#[derive(Serialize)]
struct FormatFileJson {
    path: String,
    changed: bool,
    error: Option<String>,
}

/// Run the format command.
pub fn run_format(args: FormatArgs) -> Result<(), CliError> {
    let output = args.output;
    let workspace = match WorkspaceCrates::discover(args.workspace) {
        Ok(workspace) => workspace,
        Err(error) => {
            if output.is_json() {
                output.print_json(&FormatJsonReport {
                    dry_run: args.dry_run,
                    formatted_count: 0,
                    unchanged_count: 0,
                    error_count: 1,
                    files: Vec::new(),
                    errors: vec![error.to_string()],
                })?;
                return Err(CliError::Exit(1));
            }
            return Err(error);
        },
    };
    let show_text = !output.is_json();

    if show_text && !workspace.print_discovery(ui::Ui::print_format_header) {
        return workspace.require_non_empty_selection();
    }

    if let Err(error) = workspace.require_non_empty_selection() {
        if output.is_json() {
            output.print_json(&FormatJsonReport {
                dry_run: args.dry_run,
                formatted_count: 0,
                unchanged_count: 0,
                error_count: 1,
                files: Vec::new(),
                errors: vec![error.to_string()],
            })?;
            return Err(CliError::Exit(1));
        }
        return Err(error);
    }

    let mut total_formatted = 0;
    let mut total_unchanged = 0;
    let mut errors: Vec<FormatError> = Vec::new();
    let mut json_errors: Vec<String> = Vec::new();
    let mut files = Vec::new();

    let pb = if show_text {
        ui::Ui::create_progress_bar(workspace.crates.len() as u64, "Formatting crates...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    for krate in &workspace.crates {
        pb.set_message(format!("Formatting {}", krate.name));
        let results = match format_crate(krate, args.all, args.dry_run) {
            Ok(results) => results,
            Err(error) => {
                if output.is_json() {
                    let message = relative_format_message(
                        &error.to_string(),
                        &workspace.workspace_info.root_dir,
                    );
                    json_errors.push(format!("{}: {}", krate.name, message));
                    pb.inc(1);
                    continue;
                }
                return Err(CliError::Other(error.to_string()));
            },
        };

        for result in results {
            let json_path = relative_format_path(&result.path, &workspace.workspace_info.root_dir);
            files.push(FormatFileJson {
                path: json_path.clone(),
                changed: result.changed,
                error: result.error.clone(),
            });

            if let Some(error) = result.error {
                json_errors.push(format!("{json_path}: {error}"));
                errors.push(FormatError {
                    path: result.path,
                    help: error,
                });
            } else if result.changed {
                total_formatted += 1;
                if show_text {
                    pb.suspend(|| {
                        let display_path = std::env::current_dir()
                            .ok()
                            .and_then(|cwd| result.path.strip_prefix(&cwd).ok())
                            .unwrap_or(&result.path);

                        if args.dry_run {
                            ui::Ui::print_would_format(display_path);
                            if let Some(diff) = &result.diff_info {
                                diff.print();
                            }
                        } else {
                            ui::Ui::print_formatted(display_path);
                        }
                    });
                }
            } else {
                total_unchanged += 1;
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    if output.is_json() {
        let error_count = json_errors.len();
        output.print_json(&FormatJsonReport {
            dry_run: args.dry_run,
            formatted_count: total_formatted,
            unchanged_count: total_unchanged,
            error_count,
            files,
            errors: json_errors,
        })?;
        if error_count > 0 {
            return Err(CliError::Exit(1));
        }
        return Ok(());
    }

    if errors.is_empty() {
        if args.dry_run && total_formatted > 0 {
            DryRunSummary::Format {
                formatted: total_formatted,
            }
            .print();
        } else {
            ui::Ui::print_format_summary(total_formatted, total_unchanged);
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

fn relative_format_path(path: &Path, base: &Path) -> String {
    crate::utils::paths::relative_slash_path(path, base)
}

fn relative_format_message(message: &str, base: &Path) -> String {
    crate::utils::paths::relative_slash_message(message, base)
}

/// Format all FTL files for a crate.
pub(crate) fn format_crate(
    krate: &CrateInfo,
    all_locales: bool,
    check_only: bool,
) -> Result<Vec<FormatResult>> {
    let ctx = LocaleContext::from_crate(krate, all_locales)?;
    if !ctx.assets_dir.is_dir() {
        return Ok(vec![FormatResult::error(
            &ctx.assets_dir,
            format!(
                "assets_dir for {} is missing or not a directory",
                krate.name
            ),
        )]);
    }

    let fallback_dir = ctx.locale_dir(&ctx.fallback);
    if !fallback_dir.is_dir() {
        return Ok(vec![FormatResult::error(
            &fallback_dir,
            format!(
                "fallback locale directory '{}' is missing or not a directory",
                ctx.fallback
            ),
        )]);
    }

    let mut results = Vec::new();

    if all_locales {
        match crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir) {
            Ok(issues) => {
                results.extend(issues.into_iter().map(|issue| {
                    FormatResult::error(
                        &issue.path,
                        format!(
                            "locale directory '{}' is missing or not a directory",
                            issue.locale
                        ),
                    )
                }));
            },
            Err(error) => results.push(FormatResult::error(&ctx.assets_dir, error.to_string())),
        }
    }

    for locale in &ctx.locales {
        let locale_dir = ctx.locale_dir(locale);
        if !locale_dir.is_dir() {
            results.push(FormatResult::error(
                &locale_dir,
                format!("locale directory '{locale}' is missing or not a directory"),
            ));
            continue;
        }

        // Format main + namespaced files for this crate.
        let ftl_files = CrateFtlLayout::from_assets_dir(&ctx.assets_dir, locale, &ctx.crate_name)
            .discover_files()?;
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

    let (resource, errors) = es_fluent_generate::ftl::parse_ftl_content(content.clone());
    if !errors.is_empty() {
        return FormatResult::error(
            path,
            format!(
                "Refusing to format file with parse errors: {}",
                es_fluent_generate::ftl::format_parse_errors(&errors)
            ),
        );
    }

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

    use crate::test_fixtures::{CARGO_TOML, HELLO_FTL, I18N_TOML, LIB_RS, UI_UNSORTED_FTL};

    fn write_test_crate(temp_dir: &Path) -> CrateInfo {
        let src_dir = temp_dir.join("src");
        let assets_dir = temp_dir.join("i18n/en");
        std::fs::create_dir_all(&src_dir).expect("create src");
        std::fs::create_dir_all(&assets_dir).expect("create assets");
        std::fs::create_dir_all(assets_dir.join("test-app")).expect("create namespace dir");

        let config_path = temp_dir.join("i18n.toml");
        std::fs::write(&config_path, I18N_TOML).expect("write i18n.toml");

        // Main file unchanged.
        std::fs::write(assets_dir.join("test-app.ftl"), HELLO_FTL).expect("write main ftl");

        // Namespaced file intentionally unsorted.
        std::fs::write(assets_dir.join("test-app/ui.ftl"), UI_UNSORTED_FTL)
            .expect("write namespaced ftl");

        CrateInfo {
            name: es_fluent_runner::PackageName::try_new("test-app").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(temp_dir.to_path_buf()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(config_path),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                temp_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    fn write_workspace_files(temp_dir: &Path) {
        std::fs::create_dir_all(temp_dir.join("src")).expect("create src");
        std::fs::write(temp_dir.join("Cargo.toml"), CARGO_TOML).expect("write Cargo.toml");
        std::fs::write(temp_dir.join("src/lib.rs"), LIB_RS).expect("write lib.rs");
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

        let namespaced_path = temp.path().join("i18n/en/test-app/ui.ftl");
        let namespaced_suffix = Path::new("test-app").join("ui.ftl");
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
        let namespaced_path = temp.path().join("i18n/en/test-app/ui.ftl");
        let before = std::fs::read_to_string(&namespaced_path).expect("read before");

        let results = format_crate(&krate, false, true).expect("dry run format");
        let namespaced_suffix = Path::new("test-app").join("ui.ftl");
        let namespaced_result = results
            .iter()
            .find(|r| r.path.ends_with(&namespaced_suffix))
            .expect("namespaced result exists");

        assert!(namespaced_result.changed);
        assert!(namespaced_result.diff_info.is_some());

        let after = std::fs::read_to_string(&namespaced_path).expect("read after");
        assert_eq!(before, after, "dry run should not write files");
    }

    #[test]
    fn run_format_dry_run_and_real_cover_command_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_workspace_files(temp.path());
        write_test_crate(temp.path());
        let namespaced_path = temp.path().join("i18n/en/test-app/ui.ftl");
        let before = std::fs::read_to_string(&namespaced_path).expect("read before");

        let dry_run = run_format(FormatArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            dry_run: true,
            output: OutputFormat::Text,
        });
        assert!(dry_run.is_ok());
        let after_dry_run = std::fs::read_to_string(&namespaced_path).expect("read after dry-run");
        assert_eq!(before, after_dry_run);

        let real = run_format(FormatArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: None,
            },
            all: false,
            dry_run: false,
            output: OutputFormat::Text,
        });
        assert!(real.is_ok());

        let after_real = std::fs::read_to_string(&namespaced_path).expect("read after real");
        assert_ne!(before, after_real);
        assert!(after_real.starts_with("alpha = A\nzeta = Z"));
    }

    #[test]
    fn run_format_errors_when_package_filter_matches_nothing() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_workspace_files(temp.path());
        write_test_crate(temp.path());

        let result = run_format(FormatArgs {
            workspace: WorkspaceArgs {
                path: Some(temp.path().to_path_buf()),
                package: Some("missing-package".to_string()),
            },
            all: false,
            dry_run: false,
            output: OutputFormat::Text,
        });

        assert!(
            matches!(result, Err(CliError::Other(message)) if message.contains("missing-package"))
        );
    }

    #[test]
    fn format_crate_errors_when_fallback_locale_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = write_test_crate(temp.path());
        let fallback_dir = temp.path().join("i18n/en");
        std::fs::remove_dir_all(&fallback_dir).expect("remove fallback dir");
        std::fs::write(&fallback_dir, "not a directory\n").expect("write fallback file");

        let results = format_crate(&krate, false, false).expect("format crate");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, fallback_dir);
        assert!(
            results[0]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("not a directory"))
        );
    }

    #[test]
    fn format_crate_errors_when_assets_dir_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = write_test_crate(temp.path());
        let assets_dir = temp.path().join("i18n");
        std::fs::remove_dir_all(&assets_dir).expect("remove assets dir");
        std::fs::write(&assets_dir, "not a directory\n").expect("write assets file");

        let results = format_crate(&krate, false, false).expect("format crate");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, assets_dir);
        assert!(
            results[0]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("assets_dir for test-app"))
        );
    }

    #[test]
    fn format_crate_all_reports_locale_named_asset_path_as_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = write_test_crate(temp.path());
        let locale_file = temp.path().join("i18n/fr");
        std::fs::write(&locale_file, "not a directory\n").expect("write locale file");

        let results = format_crate(&krate, true, false).expect("format crate");

        let error = results
            .iter()
            .find(|result| result.path == locale_file)
            .and_then(|result| result.error.as_deref())
            .expect("locale file should be reported as an error");
        assert!(error.contains("locale directory 'fr'"));
        assert!(error.contains("not a directory"));
    }

    #[test]
    fn format_ftl_file_covers_read_empty_and_parse_error_paths() {
        let temp = tempfile::tempdir().expect("tempdir");

        let missing = temp.path().join("missing.ftl");
        let missing_result = format_ftl_file(&missing, false);
        assert!(missing_result.error.is_some());

        let empty = temp.path().join("empty.ftl");
        std::fs::write(&empty, "   \n").expect("write empty");
        let empty_result = format_ftl_file(&empty, false);
        assert!(!empty_result.changed);
        assert!(empty_result.error.is_none());

        let invalid = temp.path().join("invalid.ftl");
        std::fs::write(&invalid, "zeta = { $name\nalpha = A\n").expect("write invalid");
        let partial = format_ftl_file(&invalid, true);
        assert!(!partial.changed);
        assert!(partial.diff_info.is_none());
        assert!(
            partial
                .error
                .as_deref()
                .is_some_and(|error| error.contains("Refusing to format file with parse errors"))
        );
    }

    #[test]
    fn relative_format_path_strips_workspace_paths_for_json() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ftl_path = temp.path().join("i18n/en/test-app.ftl");
        std::fs::create_dir_all(ftl_path.parent().expect("ftl parent")).expect("create ftl parent");
        std::fs::write(&ftl_path, "hello = Hello\n").expect("write ftl");

        let relative = relative_format_path(&ftl_path, temp.path());

        assert_eq!(relative, "i18n/en/test-app.ftl");
    }

    #[test]
    fn relative_format_message_strips_workspace_paths_for_json_errors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let message = format!(
            "Expected FTL path to be a file: {}",
            temp.path().join("i18n/en/test-app.ftl").display()
        );

        let normalized = relative_format_message(&message, temp.path());

        assert_eq!(
            normalized,
            "Expected FTL path to be a file: i18n/en/test-app.ftl"
        );
    }

    #[test]
    fn format_ftl_file_returns_write_error_for_read_only_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ftl = temp.path().join("read-only.ftl");
        std::fs::write(&ftl, "zeta = Z\nalpha = A\n").expect("write ftl");

        let mut perms = std::fs::metadata(&ftl).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&ftl, perms).unwrap();

        let result = format_ftl_file(&ftl, false);

        let mut restore = std::fs::metadata(&ftl).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            restore.set_mode(0o644);
        }
        #[cfg(not(unix))]
        {
            restore.set_readonly(false);
        }
        std::fs::set_permissions(&ftl, restore).unwrap();

        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|err| err.contains("Failed to write file")),
            "expected write error, got: {:?}",
            result.error
        );
    }
}
