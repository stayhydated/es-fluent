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
    ValidationReport, find_key_span,
};
use crate::ftl::extract_variables_from_message;
use crate::generation::{prepare_monolithic_runner_crate, run_monolithic};
use crate::utils::{get_all_locales, ui};
use anyhow::{Context as _, Result};
use clap::Parser;
use es_fluent_toml::I18nConfig;
use fluent_syntax::ast;
use fluent_syntax::parser::{self, ParserError};
use indexmap::IndexMap;
use miette::{NamedSource, SourceSpan};
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
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

    /// Keys to ignore during validation. Can be specified multiple times
    /// (e.g., --ignore a --ignore b) or comma-separated (e.g., --ignore a,b).
    #[arg(long, value_delimiter = ',')]
    pub ignore: Vec<String>,
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
    let ignore_keys: HashSet<String> = args.ignore.into_iter().collect();

    // Prepare monolithic temp crate once for all checks
    prepare_monolithic_runner_crate(&workspace.workspace_info)
        .map_err(|e| CliError::Other(e.to_string()))?;

    // First pass: collect all expected keys from all crates to validate ignore list
    let temp_dir = workspace.workspace_info.root_dir.join(".es-fluent");
    let mut all_known_keys: HashSet<String> = HashSet::new();

    let pb = ui::create_progress_bar(workspace.valid.len() as u64, "Collecting keys...");

    for krate in &workspace.valid {
        pb.set_message(format!("Scanning {}", krate.name));
        run_monolithic(&workspace.workspace_info, "check", &krate.name, &[])
            .map_err(|e| CliError::Other(e.to_string()))?;
        if let Ok(expected_keys) = read_inventory_file(&temp_dir, &krate.name) {
            all_known_keys.extend(expected_keys.into_keys());
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    // Validate that all ignore keys are known
    if !ignore_keys.is_empty() {
        let mut unknown_keys: Vec<&String> = ignore_keys
            .iter()
            .filter(|k| !all_known_keys.contains(*k))
            .collect();

        if !unknown_keys.is_empty() {
            // Sort for deterministic error messages
            unknown_keys.sort();

            return Err(CliError::Other(format!(
                "Unknown keys passed to --ignore: {}",
                unknown_keys
                    .iter()
                    .map(|k| format!("'{}'", k))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    // Second pass: validate FTL files
    let mut all_issues: Vec<ValidationIssue> = Vec::new();

    let pb = ui::create_progress_bar(workspace.valid.len() as u64, "Checking crates...");

    for krate in &workspace.valid {
        pb.set_message(format!("Checking {}", krate.name));

        match validate_crate(
            krate,
            &workspace.workspace_info.root_dir,
            &temp_dir,
            args.all,
            &ignore_keys,
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
    ignore_keys: &HashSet<String>,
) -> Result<Vec<ValidationIssue>> {
    // Read the inventory that was already collected in the first pass
    let mut expected_keys = read_inventory_file(temp_dir, &krate.name)?;

    // Filter out ignored keys
    for key in ignore_keys {
        expected_keys.shift_remove(key);
    }

    // Validate FTL files against expected keys
    validate_ftl_files(krate, workspace_root, &expected_keys, check_all)
}

/// Read inventory data from the generated inventory.json file.
fn read_inventory_file(
    temp_dir: &std::path::Path,
    crate_name: &str,
) -> Result<IndexMap<String, KeyInfo>> {
    let inventory_path = temp_dir
        .join("metadata")
        .join(crate_name)
        .join("inventory.json");
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

/// Result of loading an FTL file for a locale.
enum LocaleLoadResult {
    /// File doesn't exist.
    NotFound,
    /// Failed to read the file.
    ReadError(String),
    /// Successfully loaded.
    Loaded {
        content: String,
        resource: ast::Resource<String>,
        parse_errors: Vec<ParserError>,
    },
}

/// Load an FTL file for a locale.
fn load_locale_ftl(assets_dir: &Path, locale: &str, crate_name: &str) -> LocaleLoadResult {
    let ftl_file = assets_dir.join(locale).join(format!("{}.ftl", crate_name));

    if !ftl_file.exists() {
        return LocaleLoadResult::NotFound;
    }

    let content = match fs::read_to_string(&ftl_file) {
        Ok(c) => c,
        Err(e) => return LocaleLoadResult::ReadError(e.to_string()),
    };

    let (resource, parse_errors) = match parser::parse(content.clone()) {
        Ok(res) => (res, vec![]),
        Err((res, errors)) => (res, errors),
    };

    LocaleLoadResult::Loaded {
        content,
        resource,
        parse_errors,
    }
}

/// Validate FTL files against expected keys using fluent-syntax directly.
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
        let ftl_abs_path = assets_dir.join(locale).join(format!("{}.ftl", krate.name));
        let ftl_relative_path = to_relative_path(&ftl_abs_path, workspace_root);

        let ftl_url = format!("file://{}", ftl_abs_path.display());
        let ftl_header_link = Link::new(&ftl_relative_path, &ftl_url).to_string();

        match load_locale_ftl(&assets_dir, locale, &krate.name) {
            LocaleLoadResult::NotFound => {
                issues.extend(missing_file_issues(
                    expected_keys,
                    locale,
                    &krate.name,
                    &ftl_header_link,
                ));
                continue;
            },
            LocaleLoadResult::ReadError(err) => {
                issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                    src: NamedSource::new(ftl_header_link, String::new()),
                    span: SourceSpan::new(0_usize.into(), 1_usize),
                    locale: locale.clone(),
                    help: format!("Failed to read file: {}", err),
                }));
                continue;
            },
            LocaleLoadResult::Loaded {
                content,
                resource,
                parse_errors,
            } => {
                let ctx = ValidationContext {
                    expected_keys,
                    workspace_root,
                    manifest_dir: &krate.manifest_dir,
                };

                issues.extend(validate_loaded_ftl(
                    &content,
                    &resource,
                    &parse_errors,
                    locale,
                    &ftl_relative_path,
                    &ctx,
                ));
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

/// Validate a loaded FTL file against expected keys.
/// Validate a loaded FTL file against expected keys.
fn validate_loaded_ftl(
    content: &str,
    resource: &ast::Resource<String>,
    parse_errors: &[ParserError],
    locale: &str,
    file_name: &str,
    ctx: &ValidationContext,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut keys_with_syntax_errors: HashSet<String> = HashSet::new();

    // Calculate header link (with absolute path target but relative path text)
    let ftl_abs_path = ctx.workspace_root.join(file_name);
    let ftl_header_url = format!("file://{}", ftl_abs_path.display());
    // file_name here is expected to be relative path (passed from caller)
    let ftl_header_link = Link::new(file_name, &ftl_header_url).to_string();

    // Convert parse errors to issues
    for err in parse_errors {
        issues.push(parser_error_to_issue(
            err,
            content,
            locale,
            &ftl_header_link,
            &mut keys_with_syntax_errors,
        ));
    }

    // Scan Junk entries to find keys with errors
    for entry in &resource.body {
        if let ast::Entry::Junk { content: junk } = entry
            && let Some(key) = extract_key_from_junk(junk)
        {
            keys_with_syntax_errors.insert(key);
        }
    }

    // Build map of actual keys and their variables
    let actual_keys: IndexMap<String, HashSet<String>> = resource
        .body
        .iter()
        .filter_map(|entry| match entry {
            ast::Entry::Message(msg) => {
                Some((msg.id.name.clone(), extract_variables_from_message(msg)))
            },
            _ => None,
        })
        .collect();

    // Check for missing keys and variables
    for (key, key_info) in ctx.expected_keys {
        // Skip keys that have syntax errors - they're already reported
        if keys_with_syntax_errors.contains(key) {
            continue;
        }

        let Some(actual_vars) = actual_keys.get(key) else {
            // Key is missing
            issues.push(ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new(ftl_header_link.clone(), content.to_string()),
                key: key.clone(),
                locale: locale.to_string(),
                help: format!("Add translation for '{}' in {}", key, ftl_header_link),
            }));
            continue;
        };

        // Check for missing variables
        for var in &key_info.variables {
            if actual_vars.contains(var) {
                continue;
            }
            let span = find_key_span(content, key)
                .unwrap_or_else(|| SourceSpan::new(0_usize.into(), 1_usize));

            // Build help message with source location if available
            let help = match (&key_info.source_file, key_info.source_line) {
                (Some(file), Some(line)) => {
                    let file_path = Path::new(file);
                    let abs_file = if file_path.is_absolute() {
                        file_path.to_path_buf()
                    } else {
                        ctx.manifest_dir.join(file_path)
                    };

                    // We still want relative path for display text (relative to workspace if possible usually, or crate relative)
                    // existing logic used to_relative_path(Path::new(file), workspace_root)
                    // If file is "src/lib.rs", it's relative to crate. But we want relative to workspace?
                    // to_relative_path expects absolute or correct relative base.
                    // Let's use abs_file for to_relative_path to be safe.
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
                src: NamedSource::new(ftl_header_link.clone(), content.to_string()),
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

/// Convert a fluent-syntax ParserError to a ValidationIssue.
fn parser_error_to_issue(
    err: &ParserError,
    content: &str,
    locale: &str,
    display_name: &str,
    keys_with_syntax_errors: &mut HashSet<String>,
) -> ValidationIssue {
    // Try to extract message key from the junk slice if available
    if let Some(ref slice) = err.slice {
        let junk_content = &content[slice.clone()];
        if let Some(key) = extract_key_from_junk(junk_content) {
            keys_with_syntax_errors.insert(key);
        }
    }

    // Calculate span from ParserError position
    let span_len = if err.pos.end > err.pos.start {
        err.pos.end - err.pos.start
    } else {
        1
    };

    ValidationIssue::SyntaxError(FtlSyntaxError {
        src: NamedSource::new(display_name, content.to_string()),
        span: SourceSpan::new(err.pos.start.into(), span_len),
        locale: locale.to_string(),
        help: err.kind.to_string(),
    })
}

/// Try to extract a message key from junk content.
/// Junk typically starts with the message identifier like "message-key = ..."
fn extract_key_from_junk(junk: &str) -> Option<String> {
    static KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]+").unwrap());

    KEY_REGEX
        .find(junk.trim_start())
        .map(|m| m.as_str().to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_key_span() {
        let source = "## Comment\nhello = Hello\nworld = World";
        let span = find_key_span(source, "hello").unwrap();
        assert_eq!(span.offset(), 11);
        assert_eq!(span.len(), 5);
    }

    #[test]
    fn test_find_key_span_not_found() {
        let source = "hello = Hello";
        let span = find_key_span(source, "goodbye");
        assert!(span.is_none());
    }

    #[test]
    fn test_extract_key_from_junk() {
        assert_eq!(
            extract_key_from_junk("my-key = some value"),
            Some("my-key".to_string())
        );
        assert_eq!(
            extract_key_from_junk("  spaced-key = value"),
            Some("spaced-key".to_string())
        );
        assert_eq!(extract_key_from_junk("# comment"), None);
        assert_eq!(extract_key_from_junk(""), None);
    }

    #[test]
    fn test_extract_variables() {
        let content = "hello = Hello { $name }, you have { $count } messages";
        let resource = parser::parse(content.to_string()).unwrap();

        if let ast::Entry::Message(msg) = &resource.body[0] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("name"));
            assert!(vars.contains("count"));
            assert_eq!(vars.len(), 2);
        } else {
            panic!("Expected a message");
        }
    }
}
