//! Format command for sorting FTL entries alphabetically (A-Z).
//!
//! This module provides functionality to format FTL files by sorting
//! message keys alphabetically while preserving group comments.

use crate::discovery::discover_crates;
use crate::errors::{CliError, FormatError, FormatReport};
use crate::types::CrateInfo;
use anyhow::{Context as _, Result};
use colored::Colorize as _;
use es_fluent_toml::I18nConfig;
use fluent_syntax::{ast, parser, serializer};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const PREFIX: &str = "[es-fluent]";

/// Arguments for the format command.
#[derive(clap::Parser, Debug)]
pub struct FormatArgs {
    /// Path to the crate or workspace root (defaults to current directory).
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package).
    #[arg(short = 'P', long)]
    pub package: Option<String>,

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
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    println!(
        "{} {}",
        PREFIX.cyan().bold(),
        "Fluent FTL Formatter".dimmed()
    );

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

    let mut total_formatted = 0;
    let mut total_unchanged = 0;
    let mut errors: Vec<FormatError> = Vec::new();

    for krate in &crates {
        let results = format_crate(krate, args.all, args.dry_run)?;

        for result in results {
            if let Some(error) = result.error {
                errors.push(FormatError {
                    path: result.path,
                    help: error,
                });
            } else if result.changed {
                total_formatted += 1;
                if args.dry_run {
                    println!(
                        "{} {} {}",
                        PREFIX.yellow().bold(),
                        "Would format:".yellow(),
                        result.path.display()
                    );
                } else {
                    println!(
                        "{} {} {}",
                        PREFIX.green().bold(),
                        "Formatted:".green(),
                        result.path.display()
                    );
                }
            } else {
                total_unchanged += 1;
            }
        }
    }

    if errors.is_empty() {
        if args.dry_run && total_formatted > 0 {
            println!(
                "{} {} {} file(s) would be formatted",
                PREFIX.yellow().bold(),
                "Dry run:".yellow(),
                total_formatted
            );
        } else {
            println!(
                "{} {} {} formatted, {} unchanged",
                PREFIX.green().bold(),
                "Done:".green(),
                total_formatted,
                total_unchanged
            );
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

    let formatted = sort_ftl_resource(&resource);
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

/// Sort an FTL resource's entries alphabetically.
///
/// The sorting preserves group comments (## Comment) by associating them
/// with the messages that follow, then sorting by message key.
fn sort_ftl_resource(resource: &ast::Resource<String>) -> String {
    // Group entries: each group starts with optional comments and contains messages/terms
    let mut groups: BTreeMap<String, Vec<ast::Entry<String>>> = BTreeMap::new();
    let mut current_comments: Vec<ast::Entry<String>> = Vec::new();
    let mut standalone_comments: Vec<ast::Entry<String>> = Vec::new();

    for entry in &resource.body {
        match entry {
            ast::Entry::GroupComment(_) => {
                // Group comments start a new group
                // If we have pending comments with no message, save them as standalone
                standalone_comments.append(&mut current_comments);
                current_comments.push(entry.clone());
            },
            ast::Entry::ResourceComment(_) => {
                // Resource comments go at the top
                standalone_comments.push(entry.clone());
            },
            ast::Entry::Comment(_) => {
                // Regular comments attach to the next message
                current_comments.push(entry.clone());
            },
            ast::Entry::Message(msg) => {
                let key = msg.id.name.clone();
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                groups.insert(key, entries);
            },
            ast::Entry::Term(term) => {
                let key = format!("-{}", term.id.name);
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                groups.insert(key, entries);
            },
            ast::Entry::Junk { .. } => {
                // Skip junk entries (parse errors)
            },
        }
    }

    // Append any remaining comments
    standalone_comments.append(&mut current_comments);

    // Build the sorted resource
    let mut sorted_body: Vec<ast::Entry<String>> = Vec::new();

    // Add standalone/resource comments first
    sorted_body.extend(standalone_comments);

    // Add sorted groups
    for (_key, entries) in groups {
        sorted_body.extend(entries);
    }

    let sorted_resource = ast::Resource { body: sorted_body };
    serializer::serialize(&sorted_resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_ftl_simple() {
        let content = "zebra = Zebra\napple = Apple\nbanana = Banana";
        let resource = parser::parse(content.to_string()).unwrap();
        let sorted = sort_ftl_resource(&resource);

        // Messages should be sorted A-Z
        let lines: Vec<&str> = sorted.lines().collect();
        assert!(
            lines.iter().position(|l| l.starts_with("apple")).unwrap()
                < lines.iter().position(|l| l.starts_with("banana")).unwrap()
        );
        assert!(
            lines.iter().position(|l| l.starts_with("banana")).unwrap()
                < lines.iter().position(|l| l.starts_with("zebra")).unwrap()
        );
    }

    #[test]
    fn test_sort_ftl_with_group_comments() {
        let content = r#"## Zebras
zebra = Zebra

## Apples
apple = Apple"#;

        let resource = parser::parse(content.to_string()).unwrap();
        let sorted = sort_ftl_resource(&resource);

        // Apple group should come before Zebra group
        let apple_pos = sorted.find("## Apples").unwrap_or(usize::MAX);
        let zebra_pos = sorted.find("## Zebras").unwrap_or(usize::MAX);
        assert!(
            apple_pos < zebra_pos,
            "Apple group should come before Zebra group"
        );
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
