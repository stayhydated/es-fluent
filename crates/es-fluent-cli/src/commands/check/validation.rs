use super::inventory::KeyInfo;
use crate::core::{
    CrateInfo, FtlSyntaxError, MissingKeyError, MissingVariableWarning, ValidationIssue,
};
use crate::ftl::LocaleContext;
use crate::ftl::extract_variables_from_message;
use crate::utils::{LoadedFtlFile, discover_and_load_ftl_files, ftl::main_ftl_path};
use anyhow::Result;
use fluent_syntax::ast;
use indexmap::IndexMap;
use miette::{NamedSource, SourceSpan};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use terminal_link::Link;

/// Context for FTL validation to reduce argument count.
struct ValidationContext<'a> {
    expected_keys: &'a IndexMap<String, KeyInfo>,
    workspace_root: &'a Path,
    manifest_dir: &'a Path,
}

fn format_terminal_link(label: &str, url: &str) -> String {
    if crate::utils::ui::terminal_links_enabled() {
        Link::new(label, url).to_string()
    } else {
        label.to_string()
    }
}

/// Validate a single crate's FTL files using already-collected inventory data.
pub(crate) fn validate_crate(
    krate: &CrateInfo,
    workspace_root: &Path,
    temp_dir: &Path,
    check_all: bool,
) -> Result<Vec<ValidationIssue>> {
    // Read the inventory that was already collected in the first pass
    let expected_keys = super::inventory::read_inventory_file(temp_dir, &krate.name)?;

    // Validate FTL files against expected keys
    validate_ftl_files(krate, workspace_root, &expected_keys, check_all)
}

/// Validate FTL files against expected keys using shared discovery logic.
fn validate_ftl_files(
    krate: &CrateInfo,
    workspace_root: &Path,
    expected_keys: &IndexMap<String, KeyInfo>,
    check_all: bool,
) -> Result<Vec<ValidationIssue>> {
    let locale_ctx = LocaleContext::from_crate(krate, check_all)?;
    let assets_dir = &locale_ctx.assets_dir;

    let mut issues = Vec::new();

    for locale in &locale_ctx.locales {
        // Use shared discovery and loading logic
        match discover_and_load_ftl_files(assets_dir, locale, &locale_ctx.crate_name) {
            Ok(loaded_files) => {
                if loaded_files.is_empty() {
                    // No FTL files found at all - treat as missing main file
                    let ftl_abs_path = main_ftl_path(assets_dir, locale, &locale_ctx.crate_name);
                    let ftl_relative_path = to_relative_path(&ftl_abs_path, workspace_root);
                    let ftl_header_link = format_terminal_link(
                        &ftl_relative_path,
                        &format!("file://{}", ftl_abs_path.display()),
                    );

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
                let ftl_abs_path = main_ftl_path(assets_dir, locale, &locale_ctx.crate_name);
                let ftl_relative_path = to_relative_path(&ftl_abs_path, workspace_root);
                let ftl_header_link = format_terminal_link(
                    &ftl_relative_path,
                    &format!("file://{}", ftl_abs_path.display()),
                );

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
        let ftl_header_link = format_terminal_link(
            &ftl_relative_path,
            &format!("file://{}", file.abs_path.display()),
        );

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
                    let file_link = format_terminal_link(&file_label, &file_url);

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
                    let file_link = format_terminal_link(&rel_file, &file_url);

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
