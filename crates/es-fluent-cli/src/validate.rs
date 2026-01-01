//! Check command for validating FTL files against inventory-registered types.
//!
//! This module provides functionality to check FTL files by:
//! - Running a temp crate that collects inventory registrations
//! - Comparing FTL files against the expected keys and variables from Rust code
//! - Reporting missing keys as errors
//! - Reporting missing variables as warnings

use crate::discovery::discover_crates;
use crate::errors::{
    CliError, FtlSyntaxError, MissingKeyError, MissingVariableWarning, ValidationIssue,
    ValidationReport,
};
use crate::generator::get_es_fluent_dep;
use crate::templates::{CargoTomlTemplate, CheckRsTemplate, GitignoreTemplate};
use crate::types::CrateInfo;
use crate::ui;
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use colored::Colorize as _;
use miette::{NamedSource, SourceSpan};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PREFIX: &str = "[es-fluent]";
const TEMP_DIR: &str = ".es-fluent";
const TEMP_CRATE_NAME: &str = "es-fluent-check";

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

    let (valid_crates, skipped_crates): (Vec<_>, Vec<_>) =
        crates.iter().partition(|k| k.has_lib_rs);

    for krate in &skipped_crates {
        ui::print_missing_lib_rs(&krate.name);
    }

    let mut all_issues: Vec<ValidationIssue> = Vec::new();

    for krate in &valid_crates {
        println!(
            "{} {} {}",
            PREFIX.cyan().bold(),
            "Checking".dimmed(),
            krate.name.green()
        );

        match check_crate(krate, args.all) {
            Ok(issues) => all_issues.extend(issues),
            Err(e) => {
                println!(
                    "{} {} {}: {}",
                    PREFIX.red().bold(),
                    "Check failed for".red(),
                    krate.name.white().bold(),
                    e
                );
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

/// Check a single crate by running a temp crate that collects inventory.
fn check_crate(krate: &CrateInfo, check_all: bool) -> Result<Vec<ValidationIssue>> {
    let temp_dir = create_temp_check_crate(krate, check_all)?;
    let output = run_check_crate(&temp_dir)?;
    parse_check_output(&output, krate)
}

/// Creates a temporary crate for checking FTL files.
fn create_temp_check_crate(krate: &CrateInfo, check_all: bool) -> Result<PathBuf> {
    let temp_dir = krate.manifest_dir.join(TEMP_DIR);
    let src_dir = temp_dir.join("src");

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    // Create .gitignore to exclude the entire directory
    fs::write(
        temp_dir.join(".gitignore"),
        GitignoreTemplate.render().unwrap(),
    )
    .context("Failed to write .es-fluent/.gitignore")?;

    let crate_ident = krate.name.replace('-', "_");

    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let es_fluent_dep = get_es_fluent_dep(&manifest_path);

    // Add fluent-syntax dependency
    let es_fluent_dep_with_syntax = format!("{}\nfluent-syntax = \"0.12\"", es_fluent_dep);

    let cargo_toml = CargoTomlTemplate {
        crate_name: TEMP_CRATE_NAME,
        parent_crate_name: &krate.name,
        es_fluent_dep: &es_fluent_dep_with_syntax,
        has_fluent_features: !krate.fluent_features.is_empty(),
        fluent_features: &krate.fluent_features,
    };
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml.render().unwrap())
        .context("Failed to write .es-fluent/Cargo.toml")?;

    let i18n_toml_path_str = krate.i18n_config_path.display().to_string();
    let check_rs = CheckRsTemplate {
        crate_ident: &crate_ident,
        i18n_toml_path: &i18n_toml_path_str,
        crate_name: &krate.name,
        check_all,
    };
    fs::write(src_dir.join("main.rs"), check_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    Ok(temp_dir)
}

/// Run the check crate and capture its output.
fn run_check_crate(temp_dir: &PathBuf) -> Result<String> {
    let manifest_path = temp_dir.join("Cargo.toml");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .env("RUSTFLAGS", "-A warnings")
        .output()
        .context("Failed to run cargo")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // The check results are in stdout, but we also want to capture any build errors
    if !output.status.success() && !stdout.contains("---ES_FLUENT_CHECK_RESULTS---") {
        bail!("Cargo build failed: {}", stderr);
    }

    Ok(stdout)
}

/// Parse the output from the check crate into ValidationIssues.
fn parse_check_output(output: &str, krate: &CrateInfo) -> Result<Vec<ValidationIssue>> {
    let mut issues = Vec::new();

    // Find the results section
    let start_marker = "---ES_FLUENT_CHECK_RESULTS---";
    let end_marker = "---ES_FLUENT_CHECK_END---";

    let start = output.find(start_marker);
    let end = output.find(end_marker);

    let (Some(start), Some(end)) = (start, end) else {
        // No results section found - might be a build error
        return Ok(issues);
    };

    let results_section = &output[start + start_marker.len()..end];

    for line in results_section.lines() {
        let line = line.trim();

        if line.starts_with("E:MISSING_KEY|") {
            // E:MISSING_KEY|locale|key|file_path
            let parts: Vec<&str> = line[14..].splitn(3, '|').collect();
            if parts.len() >= 3 {
                let locale = parts[0];
                let key = parts[1];
                let file_path = parts[2];

                // Try to read the file content for source display
                let content = fs::read_to_string(file_path).unwrap_or_default();

                issues.push(ValidationIssue::MissingKey(MissingKeyError {
                    src: NamedSource::new(format!("{}/{}.ftl", locale, krate.name), content),
                    key: key.to_string(),
                    locale: locale.to_string(),
                    help: format!(
                        "Add translation for '{}' in {}/{}.ftl",
                        key, locale, krate.name
                    ),
                }));
            }
        } else if line.starts_with("E:SYNTAX_ERROR|") {
            // E:SYNTAX_ERROR|locale|file_path|error_kind
            let parts: Vec<&str> = line[15..].splitn(3, '|').collect();
            if parts.len() >= 3 {
                let locale = parts[0];
                let file_path = parts[1];
                let error_kind = parts[2];

                let content = fs::read_to_string(file_path).unwrap_or_default();
                let file_name = format!("{}/{}.ftl", locale, krate.name);

                issues.push(ValidationIssue::SyntaxError(FtlSyntaxError {
                    src: NamedSource::new(file_name.clone(), content),
                    span: SourceSpan::new(0_usize.into(), 1_usize),
                    locale: locale.to_string(),
                    file_name,
                    help: error_kind.to_string(),
                }));
            }
        } else if line.starts_with("W:MISSING_VAR|") {
            // W:MISSING_VAR|locale|key|var|file_path
            let parts: Vec<&str> = line[14..].splitn(4, '|').collect();
            if parts.len() >= 4 {
                let locale = parts[0];
                let key = parts[1];
                let var = parts[2];
                let file_path = parts[3];

                let content = fs::read_to_string(file_path).unwrap_or_default();
                let file_name = format!("{}/{}.ftl", locale, krate.name);

                // Try to find the span for this key
                let span = find_key_span(&content, key)
                    .unwrap_or_else(|| SourceSpan::new(0_usize.into(), 1_usize));

                issues.push(ValidationIssue::MissingVariable(MissingVariableWarning {
                    src: NamedSource::new(file_name, content),
                    span,
                    variable: var.to_string(),
                    key: key.to_string(),
                    locale: locale.to_string(),
                    help: format!(
                        "The Rust code expects variable '${}' but the translation omits it",
                        var
                    ),
                }));
            }
        }
    }

    Ok(issues)
}

/// Find the byte offset and length of a key in the FTL source.
fn find_key_span(source: &str, key: &str) -> Option<SourceSpan> {
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(key) {
            if rest.starts_with(" =") || rest.starts_with('=') {
                let line_start: usize = source.lines().take(line_idx).map(|l| l.len() + 1).sum();
                let key_start = line_start + (line.len() - trimmed.len());
                return Some(SourceSpan::new(key_start.into(), key.len()));
            }
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
}
