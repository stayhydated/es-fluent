use super::super::dry_run::DryRunDiff;
use crate::core::CrateInfo;
use crate::ftl::{CrateFtlLayout, LocaleContext};
use anyhow::{Result, bail};
use fluent_syntax::{ast, serializer};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Result of syncing a single locale.
#[derive(Debug)]
pub(crate) struct SyncLocaleResult {
    /// The locale that was synced.
    pub(crate) locale: String,
    /// Number of keys added.
    pub(crate) keys_added: usize,
    /// The keys that were added.
    pub(crate) added_keys: Vec<String>,
    /// Diff info (original, new) if dry run and changed.
    pub(crate) diff_info: Option<DryRunDiff>,
}

/// Sync all FTL files for a crate.
pub(crate) fn sync_crate(
    krate: &CrateInfo,
    target_locales: Option<&HashSet<String>>,
    dry_run: bool,
    create_missing: bool,
) -> Result<Vec<SyncLocaleResult>> {
    let ctx = LocaleContext::from_crate(krate, true)?;
    let fallback_dir = ctx.locale_dir(&ctx.fallback);

    if !fallback_dir.exists() {
        return Ok(Vec::new());
    }

    // Discover all FTL files in the fallback locale (including namespaced ones)
    let fallback_files =
        CrateFtlLayout::from_assets_dir(&ctx.assets_dir, &ctx.fallback, &ctx.crate_name)
            .discover_and_load_files()?;

    if fallback_files.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let mut locales: Vec<String> = match target_locales {
        Some(targets) => targets.iter().cloned().collect(),
        None => ctx.locales.clone(),
    };
    locales.sort();

    for locale in &locales {
        // Skip the fallback locale
        if locale == &ctx.fallback {
            continue;
        }

        let locale_dir = ctx.locale_dir(locale);
        if !locale_dir.exists() {
            if !create_missing {
                continue;
            }
            if !dry_run {
                fs::create_dir_all(&locale_dir)?;
            }
        }

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

    let (existing_resource, errors) =
        es_fluent_generate::ftl::parse_ftl_content(existing_content.clone());
    if !errors.is_empty() {
        bail!(
            "Refusing to sync '{}' because it contains Fluent parse errors: {}",
            ftl_file.display(),
            es_fluent_generate::ftl::format_parse_errors(&errors)
        );
    }

    let existing_keys = crate::ftl::extract_message_keys(&existing_resource);

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

    let merged = super::merge::merge_missing_keys(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_file(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        std::fs::write(path, content).expect("write file");
    }

    fn test_crate_with_i18n(temp: &tempfile::TempDir) -> CrateInfo {
        let manifest_dir = temp.path().to_path_buf();
        let src_dir = manifest_dir.join("src");
        std::fs::create_dir_all(&src_dir).expect("create src");

        let i18n_toml = manifest_dir.join("i18n.toml");
        std::fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        CrateInfo {
            name: "test-crate".to_string(),
            manifest_dir: manifest_dir.clone(),
            src_dir,
            i18n_config_path: i18n_toml,
            ftl_output_dir: manifest_dir.join("i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    fn parse_resource(content: &str) -> ast::Resource<String> {
        fluent_syntax::parser::parse(content.to_string()).unwrap()
    }

    #[test]
    fn sync_locale_file_returns_unchanged_when_no_missing_keys() {
        let temp = tempfile::tempdir().expect("tempdir");
        let locale_dir = temp.path().join("es");
        let relative_path = PathBuf::from("test-crate.ftl");
        write_file(&locale_dir.join(&relative_path), "hello = Hola\n");

        let fallback_resource = parse_resource("hello = Hello\n");
        let fallback_keys = crate::ftl::extract_message_keys(&fallback_resource);
        let result = sync_locale_file(
            &locale_dir,
            &relative_path,
            "es",
            &fallback_resource,
            &fallback_keys,
            false,
        )
        .expect("sync");

        assert_eq!(result.locale, "es");
        assert_eq!(result.keys_added, 0);
        assert!(result.added_keys.is_empty());
        assert!(result.diff_info.is_none());
    }

    #[test]
    fn sync_locale_file_dry_run_reports_diff_without_writing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let locale_dir = temp.path().join("es");
        let relative_path = PathBuf::from("test-crate.ftl");
        let ftl_path = locale_dir.join(&relative_path);
        write_file(&ftl_path, "hello = Hola\n");
        let before = std::fs::read_to_string(&ftl_path).expect("read before");

        let fallback_resource = parse_resource("hello = Hello\nworld = World\n");
        let fallback_keys = crate::ftl::extract_message_keys(&fallback_resource);
        let result = sync_locale_file(
            &locale_dir,
            &relative_path,
            "es",
            &fallback_resource,
            &fallback_keys,
            true,
        )
        .expect("sync");

        assert_eq!(result.keys_added, 1);
        assert_eq!(result.added_keys, vec!["world".to_string()]);
        assert!(result.diff_info.is_some());

        let after = std::fs::read_to_string(&ftl_path).expect("read after");
        assert_eq!(before, after, "dry-run must not write locale file");
    }

    #[test]
    fn sync_locale_file_writes_and_creates_parent_dirs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let locale_dir = temp.path().join("es");
        let relative_path = PathBuf::from("test-crate/ui.ftl");
        let ftl_path = locale_dir.join(&relative_path);

        let fallback_resource = parse_resource("hello = Hello\n");
        let fallback_keys = crate::ftl::extract_message_keys(&fallback_resource);
        let result = sync_locale_file(
            &locale_dir,
            &relative_path,
            "es",
            &fallback_resource,
            &fallback_keys,
            false,
        )
        .expect("sync");

        assert_eq!(result.keys_added, 1);
        assert!(result.diff_info.is_none());
        assert!(
            ftl_path.exists(),
            "sync should create namespaced parent dirs"
        );
        let content = std::fs::read_to_string(&ftl_path).expect("read synced file");
        assert!(content.contains("hello = Hello"));
    }

    #[test]
    fn sync_locale_file_rejects_existing_parse_errors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let locale_dir = temp.path().join("es");
        let relative_path = PathBuf::from("test-crate.ftl");
        let ftl_path = locale_dir.join(&relative_path);
        write_file(&ftl_path, "broken = {\n");

        let fallback_resource = parse_resource("hello = Hello\n");
        let fallback_keys = crate::ftl::extract_message_keys(&fallback_resource);
        let err = sync_locale_file(
            &locale_dir,
            &relative_path,
            "es",
            &fallback_resource,
            &fallback_keys,
            false,
        )
        .expect_err("invalid locale file should fail");

        assert!(err.to_string().contains("Refusing to sync"));
        assert!(err.to_string().contains("Fluent parse errors"));
    }

    #[test]
    fn sync_crate_returns_empty_when_fallback_locale_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);
        std::fs::create_dir_all(temp.path().join("i18n/es")).expect("create non-fallback locale");

        let results = sync_crate(&krate, None, false, false).expect("sync crate");
        assert!(results.is_empty());
    }

    #[test]
    fn sync_crate_filters_target_locales_and_syncs_namespaced_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);

        // Fallback files (main + namespaced).
        write_file(
            &temp.path().join("i18n/en/test-crate.ftl"),
            "hello = Hello\nworld = World\n",
        );
        write_file(
            &temp.path().join("i18n/en/test-crate/ui.ftl"),
            "button = Button\n",
        );

        // Target locale with partial content.
        write_file(
            &temp.path().join("i18n/es/test-crate.ftl"),
            "hello = Hola\n",
        );

        // Another locale that should be ignored by target filter.
        write_file(
            &temp.path().join("i18n/fr/test-crate.ftl"),
            "hello = Salut\n",
        );

        let targets = HashSet::from(["es".to_string()]);
        let results = sync_crate(&krate, Some(&targets), false, false).expect("sync crate");

        // Only `es` should be touched, and both main + namespaced files are considered.
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.locale == "es"));
        assert!(results.iter().any(|r| r.keys_added > 0));

        let es_main = std::fs::read_to_string(temp.path().join("i18n/es/test-crate.ftl"))
            .expect("read es main");
        assert!(es_main.contains("world = World"));

        let es_ns = std::fs::read_to_string(temp.path().join("i18n/es/test-crate/ui.ftl"))
            .expect("read es namespaced");
        assert!(es_ns.contains("button = Button"));

        let fr_main = std::fs::read_to_string(temp.path().join("i18n/fr/test-crate.ftl"))
            .expect("read fr main");
        assert!(
            !fr_main.contains("world = World"),
            "fr should be untouched by target filter"
        );
    }
}
