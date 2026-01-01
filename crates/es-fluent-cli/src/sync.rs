//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

use crate::discovery::discover_crates;
use crate::errors::{CliError, SyncMissingKey};
use crate::types::CrateInfo;
use anyhow::{Context as _, Result};
use colored::Colorize as _;
use es_fluent_toml::I18nConfig;
use fluent_syntax::{ast, parser, serializer};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const PREFIX: &str = "[es-fluent]";

/// Arguments for the sync command.
#[derive(clap::Parser, Debug)]
pub struct SyncArgs {
    /// Path to the crate or workspace root (defaults to current directory).
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package).
    #[arg(short = 'P', long)]
    pub package: Option<String>,

    /// Specific locale(s) to sync to (can be specified multiple times).
    #[arg(short, long)]
    pub locale: Vec<String>,

    /// Sync to all locales (excluding the fallback language).
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would be synced without making changes.
    #[arg(long)]
    pub dry_run: bool,
}

/// Result of syncing a single locale.
#[derive(Debug)]
pub struct SyncLocaleResult {
    /// The locale that was synced.
    pub locale: String,
    /// Number of keys added.
    pub keys_added: usize,
    /// The keys that were added.
    pub added_keys: Vec<String>,
}

/// Run the sync command.
pub fn run_sync(args: SyncArgs) -> Result<(), CliError> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    println!("{} {}", PREFIX.cyan().bold(), "Fluent FTL Sync".dimmed());

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

    let target_locales: Option<HashSet<String>> = if args.all {
        None // Will sync to all locales
    } else if args.locale.is_empty() {
        println!(
            "{} {}",
            PREFIX.yellow().bold(),
            "No locales specified. Use --locale <LOCALE> or --all".yellow()
        );
        return Ok(());
    } else {
        Some(args.locale.into_iter().collect())
    };

    let mut total_keys_added = 0;
    let mut total_locales_affected = 0;
    let mut all_synced_keys: Vec<SyncMissingKey> = Vec::new();

    for krate in &crates {
        println!(
            "{} {} {}",
            PREFIX.cyan().bold(),
            "Syncing".dimmed(),
            krate.name.green()
        );

        let results = sync_crate(krate, target_locales.as_ref(), args.dry_run)?;

        for result in results {
            if result.keys_added > 0 {
                total_locales_affected += 1;
                total_keys_added += result.keys_added;

                if args.dry_run {
                    println!(
                        "{} {} {} key(s) to {}",
                        PREFIX.yellow().bold(),
                        "Would add".yellow(),
                        result.keys_added,
                        result.locale.cyan()
                    );
                } else {
                    println!(
                        "{} {} {} key(s) to {}",
                        PREFIX.green().bold(),
                        "Added".green(),
                        result.keys_added,
                        result.locale.cyan()
                    );
                }

                for key in &result.added_keys {
                    println!("  {} {}", "→".dimmed(), key);
                    all_synced_keys.push(SyncMissingKey {
                        key: key.clone(),
                        target_locale: result.locale.clone(),
                        source_locale: "fallback".to_string(),
                    });
                }
            }
        }
    }

    if total_keys_added == 0 {
        println!(
            "{} {}",
            PREFIX.green().bold(),
            "All locales are in sync!".green()
        );
        Ok(())
    } else if args.dry_run {
        println!(
            "{} {} {} key(s) across {} locale(s)",
            PREFIX.yellow().bold(),
            "Would sync".yellow(),
            total_keys_added,
            total_locales_affected
        );
        // Return as report for visibility but not as error in dry-run
        Ok(())
    } else {
        println!(
            "{} {} {} key(s) synced to {} locale(s)",
            PREFIX.green().bold(),
            "Done:".green(),
            total_keys_added,
            total_locales_affected
        );
        Ok(())
    }
}

/// Sync all FTL files for a crate.
fn sync_crate(
    krate: &CrateInfo,
    target_locales: Option<&HashSet<String>>,
    dry_run: bool,
) -> Result<Vec<SyncLocaleResult>> {
    let config = I18nConfig::read_from_path(&krate.i18n_config_path)
        .with_context(|| format!("Failed to read {}", krate.i18n_config_path.display()))?;

    let assets_dir = krate.manifest_dir.join(&config.assets_dir);
    let fallback_locale = &config.fallback_language;
    let fallback_dir = assets_dir.join(fallback_locale);

    if !fallback_dir.exists() {
        return Ok(Vec::new());
    }

    // Parse fallback locale to get reference messages
    let fallback_resource = parse_ftl_file(&fallback_dir, &krate.name)?;
    let fallback_keys = extract_message_keys(&fallback_resource);

    let mut results = Vec::new();

    // Get all locales to sync to
    let locales = get_all_locales(&assets_dir)?;

    for locale in &locales {
        // Skip the fallback locale
        if locale == fallback_locale {
            continue;
        }

        // Filter by target locales if specified
        if let Some(targets) = target_locales
            && !targets.contains(locale) {
                continue;
            }

        let locale_dir = assets_dir.join(locale);
        let result = sync_locale(
            &locale_dir,
            &krate.name,
            locale,
            &fallback_resource,
            &fallback_keys,
            dry_run,
        )?;

        results.push(result);
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
            && let Some(name) = entry.file_name().to_str() {
                locales.push(name.to_string());
            }
    }

    locales.sort();
    Ok(locales)
}

/// Parse an FTL file and return the resource.
fn parse_ftl_file(locale_dir: &Path, crate_name: &str) -> Result<ast::Resource<String>> {
    let ftl_file = locale_dir.join(format!("{}.ftl", crate_name));

    if !ftl_file.exists() {
        return Ok(ast::Resource { body: Vec::new() });
    }

    let content = fs::read_to_string(&ftl_file)?;

    if content.trim().is_empty() {
        return Ok(ast::Resource { body: Vec::new() });
    }

    match parser::parse(content) {
        Ok(res) => Ok(res),
        Err((res, _)) => Ok(res), // Use partial result
    }
}

/// Extract message keys from a resource.
fn extract_message_keys(resource: &ast::Resource<String>) -> HashSet<String> {
    resource
        .body
        .iter()
        .filter_map(|entry| {
            if let ast::Entry::Message(msg) = entry {
                Some(msg.id.name.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Sync a single locale with missing keys from the fallback.
fn sync_locale(
    locale_dir: &Path,
    crate_name: &str,
    locale: &str,
    fallback_resource: &ast::Resource<String>,
    fallback_keys: &HashSet<String>,
    dry_run: bool,
) -> Result<SyncLocaleResult> {
    let ftl_file = locale_dir.join(format!("{}.ftl", crate_name));

    // Ensure the locale directory exists
    if !locale_dir.exists()
        && !dry_run {
            fs::create_dir_all(locale_dir)?;
        }

    // Parse existing locale file
    let existing_resource = parse_ftl_file(locale_dir, crate_name)?;
    let existing_keys = extract_message_keys(&existing_resource);

    // Find missing keys
    let missing_keys: Vec<&String> = fallback_keys
        .iter()
        .filter(|k| !existing_keys.contains(*k))
        .collect();

    if missing_keys.is_empty() {
        return Ok(SyncLocaleResult {
            locale: locale.to_string(),
            keys_added: 0,
            added_keys: Vec::new(),
        });
    }

    // Build the merged resource
    let mut added_keys: Vec<String> = Vec::new();

    if !dry_run {
        let merged = merge_missing_keys(
            &existing_resource,
            fallback_resource,
            &missing_keys,
            &mut added_keys,
        );

        // Serialize and write
        let content = serializer::serialize(&merged);
        let final_content = format!("{}\n", content.trim_end());
        fs::write(&ftl_file, final_content)?;
    } else {
        added_keys = missing_keys.iter().map(|k| (*k).clone()).collect();
    }

    Ok(SyncLocaleResult {
        locale: locale.to_string(),
        keys_added: added_keys.len(),
        added_keys,
    })
}

/// Merge missing keys from the fallback into the existing resource.
fn merge_missing_keys(
    existing: &ast::Resource<String>,
    fallback: &ast::Resource<String>,
    missing_keys: &[&String],
    added_keys: &mut Vec<String>,
) -> ast::Resource<String> {
    let missing_set: HashSet<&String> = missing_keys.iter().copied().collect();

    // Group existing entries by key for preservation
    let mut entries_by_key: BTreeMap<String, Vec<ast::Entry<String>>> = BTreeMap::new();
    let mut standalone_comments: Vec<ast::Entry<String>> = Vec::new();
    let mut current_comments: Vec<ast::Entry<String>> = Vec::new();

    // Process existing entries
    for entry in &existing.body {
        match entry {
            ast::Entry::GroupComment(_) | ast::Entry::ResourceComment(_) => {
                standalone_comments.append(&mut current_comments);
                current_comments.push(entry.clone());
            },
            ast::Entry::Comment(_) => {
                current_comments.push(entry.clone());
            },
            ast::Entry::Message(msg) => {
                let key = msg.id.name.clone();
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                entries_by_key.insert(key, entries);
            },
            ast::Entry::Term(term) => {
                let key = format!("-{}", term.id.name);
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                entries_by_key.insert(key, entries);
            },
            ast::Entry::Junk { .. } => {},
        }
    }

    // Add missing entries from fallback
    let mut fallback_comments: Vec<ast::Entry<String>> = Vec::new();

    for entry in &fallback.body {
        match entry {
            ast::Entry::GroupComment(_) => {
                fallback_comments.clear();
                fallback_comments.push(entry.clone());
            },
            ast::Entry::Comment(_) => {
                fallback_comments.push(entry.clone());
            },
            ast::Entry::ResourceComment(_) => {},
            ast::Entry::Message(msg) => {
                if missing_set.contains(&msg.id.name) {
                    added_keys.push(msg.id.name.clone());
                    let mut entries = std::mem::take(&mut fallback_comments);
                    entries.push(entry.clone());
                    entries_by_key.insert(msg.id.name.clone(), entries);
                } else {
                    fallback_comments.clear();
                }
            },
            ast::Entry::Term(term) => {
                let key = format!("-{}", term.id.name);
                if missing_set.contains(&key) {
                    added_keys.push(key.clone());
                    let mut entries = std::mem::take(&mut fallback_comments);
                    entries.push(entry.clone());
                    entries_by_key.insert(key, entries);
                } else {
                    fallback_comments.clear();
                }
            },
            _ => {},
        }
    }

    // Build sorted body
    let mut body: Vec<ast::Entry<String>> = Vec::new();
    body.extend(standalone_comments);
    body.append(&mut current_comments);

    for (_key, entries) in entries_by_key {
        body.extend(entries);
    }

    ast::Resource { body }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_message_keys() {
        let content = "hello = Hello\nworld = World";
        let resource = parser::parse(content.to_string()).unwrap();
        let keys = extract_message_keys(&resource);

        assert!(keys.contains("hello"));
        assert!(keys.contains("world"));
        assert_eq!(keys.len(), 2);
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

    #[test]
    fn test_merge_missing_keys() {
        let existing_content = "hello = Hello";
        let fallback_content = "hello = Hello\nworld = World\ngoodbye = Goodbye";

        let existing = parser::parse(existing_content.to_string()).unwrap();
        let fallback = parser::parse(fallback_content.to_string()).unwrap();

        let world = "world".to_string();
        let goodbye = "goodbye".to_string();
        let missing_keys: Vec<&String> = vec![&world, &goodbye];
        let mut added = Vec::new();

        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);

        assert_eq!(added.len(), 2);
        assert!(added.contains(&"world".to_string()));
        assert!(added.contains(&"goodbye".to_string()));

        // The merged resource should have all 3 messages
        let merged_keys = extract_message_keys(&merged);
        assert_eq!(merged_keys.len(), 3);
    }
}
