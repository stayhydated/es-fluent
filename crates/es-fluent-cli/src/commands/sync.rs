//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, CrateInfo, LocaleNotFoundError, SyncMissingKey};
use crate::ftl::extract_message_keys;
use crate::utils::{discover_and_load_ftl_files, get_all_locales, ui};
use anyhow::{Context as _, Result};
use clap::Parser;
use es_fluent_toml::I18nConfig;
use fluent_syntax::{ast, parser, serializer};
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
    /// Diff info (original, new) if dry run and changed.
    pub diff_info: Option<(String, String)>,
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
                        ui::print_would_add_keys(result.keys_added, &result.locale, &krate.name);
                        if let Some((old, new)) = &result.diff_info {
                            ui::print_diff(old, new);
                        }
                    } else {
                        ui::print_added_keys(result.keys_added, &result.locale);
                        for key in &result.added_keys {
                            ui::print_synced_key(key);
                            all_synced_keys.push(SyncMissingKey {
                                key: key.clone(),
                                target_locale: result.locale.clone(),
                                source_locale: "fallback".to_string(),
                            });
                        }
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

    // Discover all FTL files in the fallback locale (including namespaced ones)
    let fallback_files =
        discover_and_load_ftl_files(&assets_dir, &config.fallback_language, &krate.name)?;

    if fallback_files.is_empty() {
        return Ok(Vec::new());
    }

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

        // Sync each FTL file (main + namespaced)
        for ftl_info in &fallback_files {
            let result = sync_locale_file(
                &locale_dir,
                &ftl_info.relative_path,
                locale,
                &ftl_info.resource,
                &ftl_info.keys,
                dry_run,
            )?;

            results.push(result);
        }
    }

    Ok(results)
}

/// Sync a single FTL file (main or namespaced) with missing keys from the fallback.
fn sync_locale_file(
    locale_dir: &Path,
    relative_ftl_path: &Path,
    locale: &str,
    fallback_resource: &ast::Resource<String>,
    fallback_keys: &HashSet<String>,
    dry_run: bool,
) -> Result<SyncLocaleResult> {
    let ftl_file = locale_dir.join(relative_ftl_path);

    // Ensure the parent directory exists (handles namespaced subdirectories)
    let parent_dir = ftl_file.parent().unwrap_or(locale_dir);
    if !parent_dir.exists() && !dry_run {
        fs::create_dir_all(parent_dir)?;
    }

    // Parse existing locale file
    // Read content first to allow diffing later
    let existing_content = if ftl_file.exists() {
        fs::read_to_string(&ftl_file)?
    } else {
        String::new()
    };

    let existing_resource = parser::parse(existing_content.clone())
        .map_err(|(res, _)| res)
        .unwrap_or_else(|res| res);

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
            diff_info: None,
        });
    }

    // Build the merged resource
    let mut added_keys: Vec<String> = Vec::new();

    let merged = merge_missing_keys(
        &existing_resource,
        fallback_resource,
        &missing_keys,
        &mut added_keys,
    );
    // Serialize and write
    let content = serializer::serialize(&merged);
    let final_content = format!("{}\n", content.trim_end());

    if !dry_run {
        fs::write(&ftl_file, &final_content)?;
    }

    // If dry run and we have changes (missing_keys was not empty), compute diff
    let diff_info = if dry_run && !missing_keys.is_empty() {
        Some((existing_content, final_content))
    } else {
        None
    };

    Ok(SyncLocaleResult {
        locale: locale.to_string(),
        keys_added: added_keys.len(),
        added_keys,
        diff_info,
    })
}

/// Classification of an FTL entry for merge operations.
enum EntryKind<'a> {
    /// Group or resource comment (section header).
    SectionComment,
    /// Regular comment.
    Comment,
    /// Message with key.
    Message(std::borrow::Cow<'a, str>),
    /// Term with key (prefixed with -).
    Term(std::borrow::Cow<'a, str>),
    /// Junk or other entries.
    Other,
}

/// Classify an FTL entry for merge operations.
fn classify_entry(entry: &ast::Entry<String>) -> EntryKind<'_> {
    use std::borrow::Cow;
    match entry {
        ast::Entry::GroupComment(_) | ast::Entry::ResourceComment(_) => EntryKind::SectionComment,
        ast::Entry::Comment(_) => EntryKind::Comment,
        ast::Entry::Message(msg) => EntryKind::Message(Cow::Borrowed(&msg.id.name)),
        ast::Entry::Term(term) => EntryKind::Term(Cow::Owned(format!("-{}", term.id.name))),
        _ => EntryKind::Other,
    }
}

/// Merge missing keys from the fallback into the existing resource.
fn merge_missing_keys(
    existing: &ast::Resource<String>,
    fallback: &ast::Resource<String>,
    missing_keys: &[&String],
    added_keys: &mut Vec<String>,
) -> ast::Resource<String> {
    let missing_set: HashSet<&String> = missing_keys.iter().copied().collect();
    let existing_groups = collect_group_comments(existing);
    let mut inserted_groups: HashSet<String> = HashSet::new();

    // Group existing entries by key for preservation
    let mut entries_by_key: BTreeMap<String, Vec<ast::Entry<String>>> = BTreeMap::new();
    let mut standalone_comments: Vec<ast::Entry<String>> = Vec::new();
    let mut current_comments: Vec<ast::Entry<String>> = Vec::new();

    // Process existing entries
    for entry in &existing.body {
        match classify_entry(entry) {
            EntryKind::SectionComment => {
                standalone_comments.append(&mut current_comments);
                current_comments.push(entry.clone());
            },
            EntryKind::Comment => {
                current_comments.push(entry.clone());
            },
            EntryKind::Message(key) => {
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                entries_by_key.insert(key.to_string(), entries);
            },
            EntryKind::Term(key) => {
                let mut entries = std::mem::take(&mut current_comments);
                entries.push(entry.clone());
                entries_by_key.insert(key.to_string(), entries);
            },
            EntryKind::Other => {},
        }
    }

    // Add missing entries from fallback
    let mut fallback_comments: Vec<ast::Entry<String>> = Vec::new();

    for entry in &fallback.body {
        match classify_entry(entry) {
            EntryKind::SectionComment => {
                // ResourceComment is skipped in original, GroupComment starts fresh
                if let ast::Entry::GroupComment(comment) = entry {
                    fallback_comments.clear();
                    let group_name = group_comment_name(comment);
                    let keep_group = group_name.as_ref().map_or(true, |name| {
                        !existing_groups.contains(name) && !inserted_groups.contains(name)
                    });
                    if keep_group {
                        fallback_comments.push(entry.clone());
                    }
                }
            },
            EntryKind::Comment => {
                fallback_comments.push(entry.clone());
            },
            EntryKind::Message(key) | EntryKind::Term(key) => {
                let key_str = key.to_string();
                if missing_set.contains(&key_str) {
                    added_keys.push(key_str.clone());
                    let mut entries = std::mem::take(&mut fallback_comments);
                    entries.push(entry.clone());
                    for entry in &entries {
                        if let ast::Entry::GroupComment(comment) = entry {
                            if let Some(name) = group_comment_name(comment) {
                                inserted_groups.insert(name);
                            }
                        }
                    }
                    entries_by_key.insert(key_str, entries);
                } else {
                    fallback_comments.clear();
                }
            },
            EntryKind::Other => {},
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

fn group_comment_name(comment: &ast::Comment<String>) -> Option<String> {
    comment
        .content
        .first()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
}

fn collect_group_comments(resource: &ast::Resource<String>) -> HashSet<String> {
    let mut groups = HashSet::new();
    for entry in &resource.body {
        if let ast::Entry::GroupComment(comment) = entry {
            if let Some(name) = group_comment_name(comment) {
                groups.insert(name);
            }
        }
    }
    groups
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
        let content = r#"hello = Hello
world = World"#;
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
    fn test_merge_missing_keys_skips_duplicate_group_comments() {
        let existing_content = r#"## CountryLabelVariants

country_label_variants-Canada = Canada
"#;
        let fallback_content = r#"## CountryLabelVariants

country_label_variants-Canada = Canada
country_label_variants-USA = Usa
"#;

        let existing = parser::parse(existing_content.to_string()).unwrap();
        let fallback = parser::parse(fallback_content.to_string()).unwrap();

        let usa = "country_label_variants-USA".to_string();
        let missing_keys: Vec<&String> = vec![&usa];
        let mut added = Vec::new();

        let merged = merge_missing_keys(&existing, &fallback, &missing_keys, &mut added);

        let content = serializer::serialize(&merged);
        assert!(
            content.contains("country_label_variants-USA"),
            "Missing key should be merged"
        );
        assert_eq!(
            content.matches("## CountryLabelVariants").count(),
            1,
            "Group comment should not be duplicated: {content}"
        );
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
