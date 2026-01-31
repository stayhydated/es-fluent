//! Check command for validating FTL files against inventory-registered types.
//!
//! This module provides functionality to check FTL files by:
//! - Running a temp crate that collects inventory registrations (expected keys/variables)
//! - Parsing FTL files directly using fluent-syntax (for proper ParserError handling)
//! - Comparing FTL files against the expected keys and variables from Rust code
//! - Reporting missing keys as errors
//! - Reporting missing variables as warnings

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{
    CliError, CrateInfo, FtlSyntaxError, MissingKeyError, MissingVariableWarning, ValidationIssue,
    ValidationReport,
};
use crate::ftl::extract_variables_from_message;
use crate::generation::{prepare_monolithic_runner_crate, run_monolithic};
use crate::utils::{
    LoadedFtlFile, discover_and_load_ftl_files, ftl::main_ftl_path, get_all_locales, ui,
};
use anyhow::{Context as _, Result};
use clap::Parser;
use es_fluent_toml::I18nConfig;
use fluent_syntax::ast;
use indexmap::IndexMap;
use miette::{NamedSource, SourceSpan};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use terminal_link::Link;

/// Expected key information from inventory (deserialized from temp crate output).
#[derive(Deserialize)]
struct ExpectedKey {
    key: String,
    variables: Vec<String>,
    /// The Rust source file where this key is defined.
    source_file: Option<String>,
    /// The line number in the Rust source file.
    source_line: Option<u32>,
}

/// Runtime info about an expected key with its variables and source location.
#[derive(Clone)]
struct KeyInfo {
    variables: HashSet<String>,
    source_file: Option<String>,
    source_line: Option<u32>,
}

/// The inventory data output from the temp crate.
#[derive(Deserialize)]
struct InventoryData {
    expected_keys: Vec<ExpectedKey>,
}

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

/// Context for FTL validation to reduce argument count.
struct ValidationContext<'a> {
    expected_keys: &'a IndexMap<String, KeyInfo>,
    workspace_root: &'a Path,
    manifest_dir: &'a Path,
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

        match validate_crate(
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

/// Validate a single crate's FTL files using already-collected inventory data.
fn validate_crate(
    krate: &CrateInfo,
    workspace_root: &Path,
    temp_dir: &Path,
    check_all: bool,
) -> Result<Vec<ValidationIssue>> {
    // Read the inventory that was already collected in the first pass
    let expected_keys = read_inventory_file(temp_dir, &krate.name)?;

    // Validate FTL files against expected keys
    validate_ftl_files(krate, workspace_root, &expected_keys, check_all)
}

/// Read inventory data from the generated inventory.json file.
fn read_inventory_file(
    temp_dir: &std::path::Path,
    crate_name: &str,
) -> Result<IndexMap<String, KeyInfo>> {
    let inventory_path = es_fluent_derive_core::get_metadata_inventory_path(temp_dir, crate_name);
    let json_str = fs::read_to_string(&inventory_path)
        .with_context(|| format!("Failed to read {}", inventory_path.display()))?;

    let data: InventoryData =
        serde_json::from_str(&json_str).context("Failed to parse inventory JSON")?;

    // Convert to IndexMap with KeyInfo for richer metadata
    let mut expected_keys = IndexMap::new();
    for key_info in data.expected_keys {
        expected_keys.insert(
            key_info.key,
            KeyInfo {
                variables: key_info.variables.into_iter().collect(),
                source_file: key_info.source_file,
                source_line: key_info.source_line,
            },
        );
    }

    Ok(expected_keys)
}

/// Validate FTL files against expected keys using shared discovery logic.
fn validate_ftl_files(
    krate: &CrateInfo,
    workspace_root: &Path,
    expected_keys: &IndexMap<String, KeyInfo>,
    check_all: bool,
) -> Result<Vec<ValidationIssue>> {
    let config = I18nConfig::read_from_path(&krate.i18n_config_path)
        .with_context(|| format!("Failed to read {}", krate.i18n_config_path.display()))?;

    let assets_dir = krate.manifest_dir.join(&config.assets_dir);

    let locales: Vec<String> = if check_all {
        get_all_locales(&assets_dir)?
    } else {
        vec![config.fallback_language.clone()]
    };

    let mut issues = Vec::new();

    for locale in &locales {
        // Use shared discovery and loading logic
        match discover_and_load_ftl_files(&assets_dir, locale, &krate.name) {
            Ok(loaded_files) => {
                if loaded_files.is_empty() {
                    // No FTL files found at all - treat as missing main file
                    let ftl_abs_path = main_ftl_path(&assets_dir, locale, &krate.name);
                    let ftl_relative_path = to_relative_path(&ftl_abs_path, workspace_root);
                    let ftl_header_link = Link::new(
                        &ftl_relative_path,
                        &format!("file://{}", ftl_abs_path.display()),
                    )
                    .to_string();

                    issues.extend(missing_file_issues(
                        expected_keys,
                        locale,
                        &krate.name,
                        &ftl_header_link,
                    ));
                    continue;
                }

                // Validate all loaded files together
                let ctx = ValidationContext {
                    expected_keys,
                    workspace_root,
                    manifest_dir: &krate.manifest_dir,
                };

                issues.extend(validate_loaded_ftl_files(loaded_files, locale, &ctx));
            },
            Err(e) => {
                // Handle discovery/loading errors
                let ftl_abs_path = main_ftl_path(&assets_dir, locale, &krate.name);
                let ftl_relative_path = to_relative_path(&ftl_abs_path, workspace_root);
                let ftl_header_link = Link::new(
                    &ftl_relative_path,
                    &format!("file://{}", ftl_abs_path.display()),
                )
                .to_string();

                issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                    src: NamedSource::new(ftl_header_link, String::new()),
                    span: SourceSpan::new(0_usize.into(), 1_usize),
                    locale: locale.clone(),
                    help: format!("Failed to discover FTL files: {}", e),
                }));
            },
        }
    }

    Ok(issues)
}

/// Generate missing key issues when an FTL file doesn't exist.
fn missing_file_issues(
    expected_keys: &IndexMap<String, KeyInfo>,
    locale: &str,
    _crate_name: &str,
    ftl_path: &str,
) -> Vec<ValidationIssue> {
    expected_keys
        .keys()
        .map(|key| {
            ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new(ftl_path, String::new()),
                key: key.clone(),
                locale: locale.to_string(),
                help: format!("Add translation for '{}' in {}", key, ftl_path),
            })
        })
        .collect()
}

/// Validate multiple loaded FTL files against expected keys.
fn validate_loaded_ftl_files(
    loaded_files: Vec<LoadedFtlFile>,
    locale: &str,
    ctx: &ValidationContext,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut all_actual_keys: IndexMap<String, (HashSet<String>, String, String)> = IndexMap::new(); // key -> (vars, file_path, header_link)

    // Process all files and collect keys
    for file in loaded_files {
        let _content = fs::read_to_string(&file.abs_path).unwrap_or_default();
        let ftl_relative_path = to_relative_path(&file.abs_path, ctx.workspace_root);
        let ftl_header_link = Link::new(
            &ftl_relative_path,
            &format!("file://{}", file.abs_path.display()),
        )
        .to_string();

        // Collect actual keys from this file
        for entry in &file.resource.body {
            if let ast::Entry::Message(msg) = entry {
                let key = msg.id.name.clone();
                let vars = extract_variables_from_message(msg);

                // Store the key with its file info
                all_actual_keys.insert(
                    key.clone(),
                    (vars, ftl_relative_path.clone(), ftl_header_link.clone()),
                );
            }
        }
    }

    // Check for missing keys and variables
    for (key, key_info) in ctx.expected_keys {
        let Some((actual_vars, _file_path, header_link)) = all_actual_keys.get(key) else {
            // Key is missing from all files - report it in the first file as a reasonable default
            let default_file_path = if let Some((_, path, link)) = all_actual_keys.values().next() {
                (path.clone(), link.clone())
            } else {
                // No files at all, this case should be handled earlier but let's provide a fallback
                (format!("{}.ftl", "unknown"), format!("{}.ftl", "unknown"))
            };

            issues.push(ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new(default_file_path.1, String::new()),
                key: key.clone(),
                locale: locale.to_string(),
                help: format!("Add translation for '{}' in {}", key, default_file_path.0),
            }));
            continue;
        };

        // Check for missing variables
        for var in &key_info.variables {
            if actual_vars.contains(var) {
                continue;
            }

            // Find the span in the actual file (this is approximate)
            let span = SourceSpan::new(0_usize.into(), 1_usize);

            // Build help message with source location if available
            let help = match (&key_info.source_file, key_info.source_line) {
                (Some(file), Some(line)) => {
                    let file_path = Path::new(file);
                    let abs_file = if file_path.is_absolute() {
                        file_path.to_path_buf()
                    } else {
                        ctx.manifest_dir.join(file_path)
                    };

                    let rel_file = to_relative_path(&abs_file, ctx.workspace_root);
                    let file_label = format!("{rel_file}:{line}");
                    let file_url = format!("file://{}", abs_file.display());
                    let file_link = Link::new(&file_label, &file_url);

                    format!("Variable '${var}' is declared at {file_link}")
                },
                (Some(file), None) => {
                    let file_path = Path::new(file);
                    let abs_file = if file_path.is_absolute() {
                        file_path.to_path_buf()
                    } else {
                        ctx.manifest_dir.join(file_path)
                    };
                    let rel_file = to_relative_path(&abs_file, ctx.workspace_root);

                    let file_url = format!("file://{}", abs_file.display());
                    let file_link = Link::new(&rel_file, &file_url);

                    format!("Variable '${var}' is declared in {file_link}")
                },
                _ => format!("Variable '${var}' is declared in Rust code"),
            };

            issues.push(ValidationIssue::MissingVariable(MissingVariableWarning {
                src: NamedSource::new(header_link.clone(), String::new()),
                span,
                variable: var.clone(),
                key: key.clone(),
                locale: locale.to_string(),
                help,
            }));
        }
    }

    issues
}

/// Helper to make a path relative to a base path (e.g. workspace root).
fn to_relative_path(path: &Path, base: &Path) -> String {
    // Try to canonicalize both for accurate diffing
    let path_canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let base_canon = fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());

    // Try to strip prefix
    if let Ok(rel) = path_canon.strip_prefix(&base_canon) {
        return rel.display().to_string();
    }

    // If straightforward strip failed, we can return the path as is or try simple path strip
    // (sometimes canonicalize fails or resolves symlinks unpredictably)
    if let Ok(rel) = path.strip_prefix(base) {
        return rel.display().to_string();
    }

    // Fallback: return absolute path or best effort
    path.display().to_string()
}
