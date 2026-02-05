use super::merge::merge_missing_keys;
use crate::core::CrateInfo;
use crate::ftl::extract_message_keys;
use crate::utils::{discover_and_load_ftl_files, get_all_locales};
use anyhow::{Context as _, Result};
use es_fluent_toml::I18nConfig;
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
    pub(super) diff_info: Option<(String, String)>,
}

/// Sync all FTL files for a crate.
pub(super) fn sync_crate(
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

/// Collect all available locales across all crates.
pub(super) fn collect_all_available_locales(crates: &[CrateInfo]) -> Result<HashSet<String>> {
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
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_collect_all_available_locales() {
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
