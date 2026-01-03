//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo, LocaleNotFoundError, SyncMissingKey};
use crate::ftl::{extract_message_keys, parse_ftl_file};
use crate::utils::{get_all_locales, ui};
use anyhow::{Context as _, Result};
use clap::Parser;
use es_fluent_toml::I18nConfig;
use fluent_syntax::{ast, serializer};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

/// Arguments for the sync command.
#[derive(Debug, Parser)]
pub struct SyncArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

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
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    ui::print_sync_header();

    let crates = workspace.crates;

    if crates.is_empty() {
        ui::print_no_crates_found();
        return Ok(());
    }

    let target_locales: Option<HashSet<String>> = if args.all {
        None // Will sync to all locales
    } else if args.locale.is_empty() {
        ui::print_no_locales_specified();
        return Ok(());
    } else {
        Some(args.locale.iter().cloned().collect())
    };

    // Validate that specified locales exist
    if let Some(ref targets) = target_locales {
        let all_available_locales = collect_all_available_locales(&crates)?;

        for locale in targets {
            if !all_available_locales.contains(locale) {
                let mut available: Vec<String> = all_available_locales.into_iter().collect();
                available.sort();
                ui::print_locale_not_found(locale, &available);
                return Err(CliError::LocaleNotFound(LocaleNotFoundError {
                    locale: locale.clone(),
                    available: available.join(", "),
                }));
            }
        }
    }

    let mut total_keys_added = 0;
    let mut total_locales_affected = 0;
    let mut all_synced_keys: Vec<SyncMissingKey> = Vec::new();

    let pb = ui::create_progress_bar(crates.len() as u64, "Syncing crates...");

    for krate in &crates {
        pb.set_message(format!("Syncing {}", krate.name));

        let results = sync_crate(krate, target_locales.as_ref(), args.dry_run)?;

        for result in results {
            if result.keys_added > 0 {
                total_locales_affected += 1;
                total_keys_added += result.keys_added;

                pb.suspend(|| {
                    if args.dry_run {
                        ui::print_would_add_keys(result.keys_added, &result.locale);
                    } else {
                        ui::print_added_keys(result.keys_added, &result.locale);
                    }

                    for key in &result.added_keys {
                        ui::print_synced_key(key);
                        all_synced_keys.push(SyncMissingKey {
                            key: key.clone(),
                            target_locale: result.locale.clone(),
                            source_locale: "fallback".to_string(),
                        });
                    }
                });
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    if total_keys_added == 0 {
        ui::print_all_in_sync();
        Ok(())
    } else if args.dry_run {
        ui::print_sync_dry_run_summary(total_keys_added, total_locales_affected);
        Ok(())
    } else {
        ui::print_sync_summary(total_keys_added, total_locales_affected);
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
    let fallback_ftl = fallback_dir.join(format!("{}.ftl", krate.name));
    let fallback_resource = parse_ftl_file(&fallback_ftl)?;
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
            && !targets.contains(locale)
        {
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
    if !locale_dir.exists() && !dry_run {
        fs::create_dir_all(locale_dir)?;
    }

    // Parse existing locale file
    let existing_resource = parse_ftl_file(&ftl_file)?;
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

/// Collect all available locales across all crates.
fn collect_all_available_locales(crates: &[CrateInfo]) -> Result<HashSet<String>> {
    let mut all_locales = HashSet::new();

    for krate in crates {
        let config = I18nConfig::read_from_path(&krate.i18n_config_path)
            .with_context(|| format!("Failed to read {}", krate.i18n_config_path.display()))?;

        let assets_dir = krate.manifest_dir.join(&config.assets_dir);
        let locales = get_all_locales(&assets_dir)?;

        for locale in locales {
            all_locales.insert(locale);
        }
    }

    Ok(all_locales)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::parser;

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

    #[test]
    fn test_collect_all_available_locales() {
        use std::path::PathBuf;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let assets = temp_dir.path().join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("en")).unwrap();
        fs::create_dir(assets.join("fr")).unwrap();
        fs::create_dir(assets.join("de")).unwrap();

        // Create a minimal i18n.toml
        let config_path = temp_dir.path().join("i18n.toml");
        fs::write(
            &config_path,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();

        let crates = vec![CrateInfo {
            name: "test-crate".to_string(),
            manifest_dir: temp_dir.path().to_path_buf(),
            src_dir: PathBuf::new(),
            i18n_config_path: config_path,
            ftl_output_dir: PathBuf::new(),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }];

        let locales = collect_all_available_locales(&crates).unwrap();

        assert!(locales.contains("en"));
        assert!(locales.contains("fr"));
        assert!(locales.contains("de"));
        assert_eq!(locales.len(), 3);
        assert!(!locales.contains("awd"));
    }
}
