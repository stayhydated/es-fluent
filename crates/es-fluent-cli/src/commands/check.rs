//! Check command for validating FTL files against inventory-registered types.
//!
//! This module provides functionality to check FTL files by:
//! - Running a temp crate that collects inventory registrations (expected keys/variables)
//! - Parsing FTL files directly using fluent-syntax (for proper ParserError handling)
//! - Comparing FTL files against the expected keys and variables from Rust code
//! - Reporting missing keys as errors
//! - Reporting missing variables as warnings

use crate::core::{
    CliError, CrateInfo, FtlSyntaxError, MissingKeyError, MissingVariableWarning, ValidationIssue,
    ValidationReport,
};
use crate::generation::{
    CargoTomlTemplate, CheckRsTemplate, create_temp_dir, get_es_fluent_dep, run_cargo_with_output,
    write_cargo_toml, write_main_rs,
};
use crate::utils::{
    discover_crates, filter_crates_by_package, get_all_locales, partition_by_lib_rs, ui,
};
use anyhow::{Context as _, Result};
use askama::Template as _;
use clap::Parser;
use es_fluent_toml::I18nConfig;
use fluent_syntax::ast;
use fluent_syntax::parser::{self, ParserError};
use miette::{NamedSource, SourceSpan};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// Expected key information from inventory (deserialized from temp crate output).
#[derive(Deserialize)]
struct ExpectedKey {
    key: String,
    variables: Vec<String>,
}

/// The inventory data output from the temp crate.
#[derive(Deserialize)]
struct InventoryData {
    expected_keys: Vec<ExpectedKey>,
}

const TEMP_CRATE_NAME: &str = "es-fluent-check";

/// Arguments for the check command.
#[derive(Debug, Parser)]
pub struct CheckArgs {
    /// Path to the crate or workspace root (defaults to current directory).
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package).
    #[arg(short = 'P', long)]
    pub package: Option<String>,

    /// Check all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,
}

/// Run the check command.
pub fn run_check(args: CheckArgs) -> Result<(), CliError> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_check_header();

    let crates = discover_crates(&path)?;
    let crates = filter_crates_by_package(crates, args.package.as_ref());

    if crates.is_empty() {
        ui::print_no_crates_found();
        return Ok(());
    }

    let (valid_crates, skipped_crates) = partition_by_lib_rs(&crates);

    for krate in &skipped_crates {
        ui::print_missing_lib_rs(&krate.name);
    }

    let mut all_issues: Vec<ValidationIssue> = Vec::new();

    for krate in &valid_crates {
        ui::print_checking(&krate.name);

        match check_crate(krate, args.all) {
            Ok(issues) => all_issues.extend(issues),
            Err(e) => {
                ui::print_check_error(&krate.name, &e.to_string());
            },
        }
    }

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

/// Check a single crate by running a temp crate to collect inventory, then validating FTL files.
fn check_crate(krate: &CrateInfo, check_all: bool) -> Result<Vec<ValidationIssue>> {
    // Step 1: Get expected keys from inventory via temp crate
    let temp_dir = create_temp_check_crate(krate)?;
    run_check_crate(&temp_dir)?;
    let expected_keys = read_inventory_file(&temp_dir)?;

    // Step 2: Parse FTL files and validate against expected keys
    validate_ftl_files(krate, &expected_keys, check_all)
}

/// Creates a temporary crate for collecting inventory data.
fn create_temp_check_crate(krate: &CrateInfo) -> Result<PathBuf> {
    let temp_dir = create_temp_dir(krate)?;

    let crate_ident = krate.name.replace('-', "_");
    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let es_fluent_dep = get_es_fluent_dep(&manifest_path, "cli");

    let cargo_toml = CargoTomlTemplate {
        crate_name: TEMP_CRATE_NAME,
        parent_crate_name: &krate.name,
        es_fluent_dep: &es_fluent_dep,
        has_fluent_features: !krate.fluent_features.is_empty(),
        fluent_features: &krate.fluent_features,
    };
    write_cargo_toml(&temp_dir, &cargo_toml.render().unwrap())?;

    let check_rs = CheckRsTemplate {
        crate_ident: &crate_ident,
        crate_name: &krate.name,
    };
    write_main_rs(&temp_dir, &check_rs.render().unwrap())?;

    Ok(temp_dir)
}

/// Run the check crate to generate inventory.json.
fn run_check_crate(temp_dir: &std::path::Path) -> Result<()> {
    run_cargo_with_output(temp_dir)?;
    Ok(())
}

/// Read inventory data from the generated inventory.json file.
fn read_inventory_file(temp_dir: &std::path::Path) -> Result<HashMap<String, HashSet<String>>> {
    let inventory_path = temp_dir.join("inventory.json");
    let json_str = fs::read_to_string(&inventory_path)
        .with_context(|| format!("Failed to read {}", inventory_path.display()))?;

    let data: InventoryData =
        serde_json::from_str(&json_str).context("Failed to parse inventory JSON")?;

    // Convert to HashMap for easier lookup
    let mut expected_keys = HashMap::new();
    for key_info in data.expected_keys {
        expected_keys.insert(key_info.key, key_info.variables.into_iter().collect());
    }

    Ok(expected_keys)
}

/// Validate FTL files against expected keys using fluent-syntax directly.
fn validate_ftl_files(
    krate: &CrateInfo,
    expected_keys: &HashMap<String, HashSet<String>>,
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
        let locale_dir = assets_dir.join(locale);
        let ftl_file = locale_dir.join(format!("{}.ftl", krate.name));

        if !ftl_file.exists() {
            // Report all keys as missing for this locale
            for key in expected_keys.keys() {
                issues.push(ValidationIssue::MissingKey(MissingKeyError {
                    src: NamedSource::new(format!("{}/{}.ftl", locale, krate.name), String::new()),
                    key: key.clone(),
                    locale: locale.clone(),
                    help: format!(
                        "Add translation for '{}' in {}/{}.ftl",
                        key, locale, krate.name
                    ),
                }));
            }
            continue;
        }

        let content = match fs::read_to_string(&ftl_file) {
            Ok(c) => c,
            Err(e) => {
                issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                    src: NamedSource::new(format!("{}/{}.ftl", locale, krate.name), String::new()),
                    span: SourceSpan::new(0_usize.into(), 1_usize),
                    locale: locale.clone(),
                    file_name: format!("{}/{}.ftl", locale, krate.name),
                    help: format!("Failed to read file: {}", e),
                }));
                continue;
            },
        };

        let file_name = format!("{}/{}.ftl", locale, krate.name);

        // Track keys that have syntax errors (found in Junk entries)
        let mut keys_with_syntax_errors: HashSet<String> = HashSet::new();

        // Parse the FTL file using fluent-syntax
        let resource = match parser::parse(content.clone()) {
            Ok(res) => res,
            Err((res, parse_errors)) => {
                // Convert ParserErrors to ValidationIssues
                for err in parse_errors {
                    issues.push(parser_error_to_issue(
                        &err,
                        &content,
                        locale,
                        &file_name,
                        &mut keys_with_syntax_errors,
                    ));
                }
                res
            },
        };

        // Also scan Junk entries to find keys with errors
        for entry in &resource.body {
            if let ast::Entry::Junk { content: junk } = entry
                && let Some(key) = extract_key_from_junk(junk)
            {
                keys_with_syntax_errors.insert(key);
            }
        }

        // Build map of actual keys and their variables in the FTL file
        let mut actual_keys: HashMap<String, HashSet<String>> = HashMap::new();
        for entry in &resource.body {
            if let ast::Entry::Message(msg) = entry {
                let key = msg.id.name.clone();
                let vars = extract_variables_from_message(msg);
                actual_keys.insert(key, vars);
            }
        }

        // Check for missing keys (but skip keys that have syntax errors)
        for (key, expected_vars) in expected_keys {
            // Skip keys that have syntax errors - they're already reported
            if keys_with_syntax_errors.contains(key) {
                continue;
            }

            if let Some(actual_vars) = actual_keys.get(key) {
                // Key exists, check for missing variables
                for var in expected_vars {
                    if !actual_vars.contains(var) {
                        let span = find_key_span(&content, key)
                            .unwrap_or_else(|| SourceSpan::new(0_usize.into(), 1_usize));

                        issues.push(ValidationIssue::MissingVariable(MissingVariableWarning {
                            src: NamedSource::new(file_name.clone(), content.clone()),
                            span,
                            variable: var.clone(),
                            key: key.clone(),
                            locale: locale.clone(),
                            help: format!(
                                "The Rust code expects variable '${}' but the translation omits it",
                                var
                            ),
                        }));
                    }
                }
            } else {
                // Key is missing
                issues.push(ValidationIssue::MissingKey(MissingKeyError {
                    src: NamedSource::new(file_name.clone(), content.clone()),
                    key: key.clone(),
                    locale: locale.clone(),
                    help: format!(
                        "Add translation for '{}' in {}/{}.ftl",
                        key, locale, krate.name
                    ),
                }));
            }
        }
    }

    Ok(issues)
}

/// Convert a fluent-syntax ParserError to a ValidationIssue.
fn parser_error_to_issue(
    err: &ParserError,
    content: &str,
    locale: &str,
    file_name: &str,
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
        src: NamedSource::new(file_name, content.to_string()),
        span: SourceSpan::new(err.pos.start.into(), span_len),
        locale: locale.to_string(),
        file_name: file_name.to_string(),
        help: err.kind.to_string(),
    })
}

/// Try to extract a message key from junk content.
/// Junk typically starts with the message identifier like "message-key = ..."
fn extract_key_from_junk(junk: &str) -> Option<String> {
    let trimmed = junk.trim_start();

    // Skip comments
    if trimmed.starts_with('#') {
        return None;
    }

    // Find the identifier (sequence of valid FTL identifier chars before '=' or whitespace)
    let mut key = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            key.push(ch);
        } else {
            break;
        }
    }

    if key.is_empty() { None } else { Some(key) }
}

/// Extract variables from a message.
fn extract_variables_from_message(msg: &ast::Message<String>) -> HashSet<String> {
    let mut variables = HashSet::new();
    if let Some(ref value) = msg.value {
        extract_variables_from_pattern(value, &mut variables);
    }
    for attr in &msg.attributes {
        extract_variables_from_pattern(&attr.value, &mut variables);
    }
    variables
}

/// Extract variables from a pattern.
fn extract_variables_from_pattern(pattern: &ast::Pattern<String>, variables: &mut HashSet<String>) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            extract_variables_from_expression(expression, variables);
        }
    }
}

/// Extract variables from an expression.
fn extract_variables_from_expression(
    expression: &ast::Expression<String>,
    variables: &mut HashSet<String>,
) {
    match expression {
        ast::Expression::Inline(inline) => {
            extract_variables_from_inline(inline, variables);
        },
        ast::Expression::Select { selector, variants } => {
            extract_variables_from_inline(selector, variables);
            for variant in variants {
                extract_variables_from_pattern(&variant.value, variables);
            }
        },
    }
}

/// Extract variables from an inline expression.
fn extract_variables_from_inline(
    inline: &ast::InlineExpression<String>,
    variables: &mut HashSet<String>,
) {
    match inline {
        ast::InlineExpression::VariableReference { id } => {
            variables.insert(id.name.clone());
        },
        ast::InlineExpression::FunctionReference { arguments, .. } => {
            for arg in &arguments.positional {
                extract_variables_from_inline(arg, variables);
            }
            for arg in &arguments.named {
                extract_variables_from_inline(&arg.value, variables);
            }
        },
        ast::InlineExpression::Placeable { expression } => {
            extract_variables_from_expression(expression, variables);
        },
        _ => {},
    }
}

/// Find the byte offset and length of a key in the FTL source.
fn find_key_span(source: &str, key: &str) -> Option<SourceSpan> {
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(key)
            && (rest.starts_with(" =") || rest.starts_with('='))
        {
            let line_start: usize = source.lines().take(line_idx).map(|l| l.len() + 1).sum();
            let key_start = line_start + (line.len() - trimmed.len());
            return Some(SourceSpan::new(key_start.into(), key.len()));
        }
    }
    None
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
