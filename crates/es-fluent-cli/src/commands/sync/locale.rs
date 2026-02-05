use super::merge::merge_missing_keys;
use crate::commands::DryRunDiff;
use crate::core::CrateInfo;
use crate::ftl::{LocaleContext, extract_message_keys};
use crate::utils::discover_and_load_ftl_files;
use anyhow::Result;
use fluent_syntax::{ast, parser, serializer};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Result of syncing a single locale.
#[derive(Debug)]
pub(super) struct SyncLocaleResult {
    /// The locale that was synced.
    pub(super) locale: String,
    /// Number of keys added.
    pub(super) keys_added: usize,
    /// The keys that were added.
    pub(super) added_keys: Vec<String>,
    /// Diff info (original, new) if dry run and changed.
    pub(super) diff_info: Option<DryRunDiff>,
}

/// Sync all FTL files for a crate.
pub(super) fn sync_crate(
    krate: &CrateInfo,
    target_locales: Option<&HashSet<String>>,
    dry_run: bool,
) -> Result<Vec<SyncLocaleResult>> {
    let ctx = LocaleContext::from_crate(krate, true)?;
    let fallback_dir = ctx.locale_dir(&ctx.fallback);

    if !fallback_dir.exists() {
        return Ok(Vec::new());
    }

    // Discover all FTL files in the fallback locale (including namespaced ones)
    let fallback_files =
        discover_and_load_ftl_files(&ctx.assets_dir, &ctx.fallback, &ctx.crate_name)?;

    if fallback_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();

    for locale in &ctx.locales {
        // Skip the fallback locale
        if locale == &ctx.fallback {
            continue;
        }

        // Filter by target locales if specified
        if let Some(targets) = target_locales
            && !targets.contains(locale)
        {
            continue;
        }

        let locale_dir = ctx.locale_dir(locale);

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
        Some(DryRunDiff::new(existing_content, final_content))
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
