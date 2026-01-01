//! Check command for checking FTL files for missing keys and variables.
//!
//! This module provides functionality to check FTL files by:
//! - Checking for missing translation keys (errors)
//! - Checking for omitted variables in translations (warnings)

use crate::discovery::discover_crates;
use crate::errors::{
    CliError, FtlSyntaxError, MissingKeyError, MissingVariableWarning, ValidationIssue,
    ValidationReport, find_message_span,
};
use crate::types::CrateInfo;
use anyhow::{Context as _, Result};
use colored::Colorize as _;
use es_fluent_toml::I18nConfig;
use fluent_syntax::{ast, parser};
use miette::NamedSource;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const PREFIX: &str = "[es-fluent]";

/// Arguments for the check command.
#[derive(clap::Parser, Debug)]
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

/// Information about a parsed FTL message.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MessageInfo {
    /// The message key.
    key: String,
    /// Variables used in this message.
    variables: HashSet<String>,
    /// The source location (line offset).
    offset: usize,
    /// Length of the message definition.
    length: usize,
}

/// Run the check command.
pub fn run_check(args: CheckArgs) -> Result<(), CliError> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    println!("{} {}", PREFIX.cyan().bold(), "Fluent FTL Checker".dimmed());

    let crates = discover_crates(&path)?;

    let crates: Vec<_> = if let Some(ref pkg) = args.package {
        crates.into_iter().filter(|c| &c.name == pkg).collect()
    } else {
        crates
    };

    if crates.is_empty() {
        println!(
            "{} {}",
            PREFIX.red().bold(),
            "No crates with i18n.toml found.".red()
        );
        return Ok(());
    }

    let mut all_issues: Vec<ValidationIssue> = Vec::new();

    for krate in &crates {
        println!(
            "{} {} {}",
            PREFIX.cyan().bold(),
            "Validating".dimmed(),
            krate.name.green()
        );

        let issues = validate_crate(krate, args.all)?;
        all_issues.extend(issues);
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
        println!("{} {}", PREFIX.green().bold(), "No issues found!".green());
        Ok(())
    } else {
        Err(CliError::Validation(ValidationReport {
            error_count,
            warning_count,
            issues: all_issues,
        }))
    }
}

/// Validate all FTL files for a crate.
fn validate_crate(krate: &CrateInfo, all_locales: bool) -> Result<Vec<ValidationIssue>> {
    let config = I18nConfig::read_from_path(&krate.i18n_config_path)
        .with_context(|| format!("Failed to read {}", krate.i18n_config_path.display()))?;

    let assets_dir = krate.manifest_dir.join(&config.assets_dir);
    let fallback_locale = &config.fallback_language;
    let fallback_dir = assets_dir.join(fallback_locale);

    if !fallback_dir.exists() {
        return Ok(Vec::new());
    }

    // Parse fallback locale to get reference messages
    let fallback_messages = parse_locale_messages(&fallback_dir, &krate.name)?;

    let mut issues = Vec::new();

    if all_locales {
        // Validate all non-fallback locales against the fallback
        let locales = get_all_locales(&assets_dir)?;

        for locale in &locales {
            if locale == fallback_locale {
                continue;
            }

            let locale_dir = assets_dir.join(locale);
            if !locale_dir.exists() {
                continue;
            }

            let locale_issues =
                validate_locale(&locale_dir, &krate.name, locale, &fallback_messages)?;
            issues.extend(locale_issues);
        }
    } else {
        // Just validate the fallback locale for syntax errors
        let fallback_issues = validate_locale_syntax(&fallback_dir, &krate.name, fallback_locale)?;
        issues.extend(fallback_issues);
    }

    Ok(issues)
}

/// Get all locale directories from the assets directory.
fn get_all_locales(assets_dir: &Path) -> Result<Vec<String>> {
    let mut locales = Vec::new();

    if !assets_dir.exists() {
        return Ok(locales);
    }

    for entry in fs::read_dir(assets_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            locales.push(name.to_string());
        }
    }

    locales.sort();
    Ok(locales)
}

/// Parse all messages from a locale directory.
fn parse_locale_messages(
    locale_dir: &Path,
    crate_name: &str,
) -> Result<HashMap<String, MessageInfo>> {
    let mut messages = HashMap::new();

    let ftl_file = locale_dir.join(format!("{}.ftl", crate_name));
    if !ftl_file.exists() {
        return Ok(messages);
    }

    let content = fs::read_to_string(&ftl_file)?;
    if content.trim().is_empty() {
        return Ok(messages);
    }

    let resource = match parser::parse(content.clone()) {
        Ok(res) => res,
        Err((res, _)) => res, // Use partial result
    };

    let mut current_offset = 0;

    for entry in &resource.body {
        if let ast::Entry::Message(msg) = entry {
            let key = msg.id.name.clone();
            let variables = extract_variables_from_message(msg);

            // Calculate offset for this message
            if let Some(span) = find_message_span(&content, &key) {
                messages.insert(
                    key.clone(),
                    MessageInfo {
                        key,
                        variables,
                        offset: span.offset(),
                        length: span.len(),
                    },
                );
            } else {
                messages.insert(
                    key.clone(),
                    MessageInfo {
                        key,
                        variables,
                        offset: current_offset,
                        length: 0,
                    },
                );
            }
        }
        // Track approximate offset
        current_offset += 50; // Rough estimate
    }

    Ok(messages)
}

/// Extract variable names from a Fluent message.
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

/// Extract variable names from a Fluent pattern.
fn extract_variables_from_pattern(pattern: &ast::Pattern<String>, variables: &mut HashSet<String>) {
    for element in &pattern.elements {
        if let ast::PatternElement::Placeable { expression } = element {
            extract_variables_from_expression(expression, variables);
        }
    }
}

/// Extract variable names from a Fluent expression.
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

/// Extract variable names from an inline expression.
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

/// Validate a locale against the fallback messages.
fn validate_locale(
    locale_dir: &Path,
    crate_name: &str,
    locale: &str,
    fallback_messages: &HashMap<String, MessageInfo>,
) -> Result<Vec<ValidationIssue>> {
    let mut issues = Vec::new();

    let ftl_file = locale_dir.join(format!("{}.ftl", crate_name));
    if !ftl_file.exists() {
        // All keys from fallback are missing
        for key in fallback_messages.keys() {
            issues.push(ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new(format!("{}/{}.ftl", locale, crate_name), String::new()),
                key: key.clone(),
                locale: locale.to_string(),
                help: format!(
                    "Add translation for '{}' in {}/{}.ftl",
                    key, locale, crate_name
                ),
            }));
        }
        return Ok(issues);
    }

    let content = fs::read_to_string(&ftl_file)?;
    let file_name = format!("{}/{}.ftl", locale, crate_name);

    if content.trim().is_empty() {
        // All keys from fallback are missing
        for key in fallback_messages.keys() {
            issues.push(ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new(file_name.clone(), content.clone()),
                key: key.clone(),
                locale: locale.to_string(),
                help: format!("Add translation for '{}' in {}", key, file_name),
            }));
        }
        return Ok(issues);
    }

    // Parse the locale file
    let resource = match parser::parse(content.clone()) {
        Ok(res) => res,
        Err((res, errors)) => {
            // Report syntax errors
            for error in &errors {
                let (offset, length) = error_span(error);
                issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                    src: NamedSource::new(file_name.clone(), content.clone()),
                    span: miette::SourceSpan::new(offset.into(), length),
                    locale: locale.to_string(),
                    file_name: file_name.clone(),
                    help: format!("{:?}", error.kind),
                }));
            }
            res
        },
    };

    // Build map of messages in this locale
    let locale_messages = parse_messages_from_resource(&resource, &content);

    // Check for missing keys
    for (key, fallback_info) in fallback_messages {
        if !locale_messages.contains_key(key) {
            issues.push(ValidationIssue::MissingKey(MissingKeyError {
                src: NamedSource::new(file_name.clone(), content.clone()),
                key: key.clone(),
                locale: locale.to_string(),
                help: format!("Add translation for '{}' in {}", key, file_name),
            }));
        } else {
            // Check for missing variables
            let locale_info = &locale_messages[key];
            for var in &fallback_info.variables {
                if !locale_info.variables.contains(var) {
                    let span = find_message_span(&content, key)
                        .unwrap_or_else(|| miette::SourceSpan::new(0_usize.into(), 1_usize));

                    issues.push(ValidationIssue::MissingVariable(MissingVariableWarning {
                        src: NamedSource::new(file_name.clone(), content.clone()),
                        span,
                        variable: var.clone(),
                        key: key.clone(),
                        locale: locale.to_string(),
                        help: format!(
                            "The fallback translation uses '${}' but this translation omits it",
                            var
                        ),
                    }));
                }
            }
        }
    }

    Ok(issues)
}

/// Validate a locale for syntax errors only.
fn validate_locale_syntax(
    locale_dir: &Path,
    crate_name: &str,
    locale: &str,
) -> Result<Vec<ValidationIssue>> {
    let mut issues = Vec::new();

    let ftl_file = locale_dir.join(format!("{}.ftl", crate_name));
    if !ftl_file.exists() {
        return Ok(issues);
    }

    let content = fs::read_to_string(&ftl_file)?;
    let file_name = format!("{}/{}.ftl", locale, crate_name);

    if content.trim().is_empty() {
        return Ok(issues);
    }

    if let Err((_res, errors)) = parser::parse(content.clone()) {
        for error in &errors {
            let (offset, length) = error_span(error);
            issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                src: NamedSource::new(file_name.clone(), content.clone()),
                span: miette::SourceSpan::new(offset.into(), length),
                locale: locale.to_string(),
                file_name: file_name.clone(),
                help: format!("{:?}", error.kind),
            }));
        }
    }

    Ok(issues)
}

/// Parse messages from a resource into a map.
fn parse_messages_from_resource(
    resource: &ast::Resource<String>,
    content: &str,
) -> HashMap<String, MessageInfo> {
    let mut messages = HashMap::new();

    for entry in &resource.body {
        if let ast::Entry::Message(msg) = entry {
            let key = msg.id.name.clone();
            let variables = extract_variables_from_message(msg);

            let (offset, length) = if let Some(span) = find_message_span(content, &key) {
                (span.offset(), span.len())
            } else {
                (0, 0)
            };

            messages.insert(
                key.clone(),
                MessageInfo {
                    key,
                    variables,
                    offset,
                    length,
                },
            );
        }
    }

    messages
}

/// Get the span for a parser error.
fn error_span(error: &fluent_syntax::parser::ParserError) -> (usize, usize) {
    let start = error.pos.start;
    let end = error.pos.end;
    (start, end.saturating_sub(start).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_variables_simple() {
        let content = "hello = Hello, { $name }!";
        let resource = parser::parse(content.to_string()).unwrap();

        if let ast::Entry::Message(msg) = &resource.body[0] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("name"));
        } else {
            panic!("Expected message");
        }
    }

    #[test]
    fn test_extract_variables_select() {
        let content = r#"photos = { $count ->
    [one] { $user } added a photo
   *[other] { $user } added { $count } photos
}"#;
        let resource = parser::parse(content.to_string()).unwrap();

        if let ast::Entry::Message(msg) = &resource.body[0] {
            let vars = extract_variables_from_message(msg);
            assert!(vars.contains("count"));
            assert!(vars.contains("user"));
        } else {
            panic!("Expected message");
        }
    }

    #[test]
    fn test_get_all_locales() {
        let temp_dir = tempfile::tempdir().unwrap();
        let assets = temp_dir.path();

        fs::create_dir(assets.join("en")).unwrap();
        fs::create_dir(assets.join("fr")).unwrap();
        fs::create_dir(assets.join("de")).unwrap();

        let locales = get_all_locales(assets).unwrap();
        assert_eq!(locales, vec!["de", "en", "fr"]);
    }
}
