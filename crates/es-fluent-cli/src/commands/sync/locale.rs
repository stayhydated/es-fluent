use super::super::dry_run::DryRunDiff;
use crate::core::CrateInfo;
use crate::ftl::{CrateFtlLayout, LocaleContext};
use anyhow::{Result, bail};
use fluent_syntax::{ast, serializer};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Result of syncing a single locale.
#[derive(Debug)]
pub(crate) struct SyncLocaleResult {
    /// The locale that was synced.
    pub(crate) locale: String,
    /// Whether the locale directory was created.
    pub(crate) locale_created: bool,
    /// Number of keys added.
    pub(crate) keys_added: usize,
    /// The keys that were added.
    pub(crate) added_keys: Vec<String>,
    /// Diff info (original, new) if dry run and changed.
    pub(crate) diff_info: Option<DryRunDiff>,
}

struct SyncLocalePlan {
    locale: String,
    locale_dir: PathBuf,
    locale_created: bool,
}

struct SyncCratePlan {
    fallback_files: Vec<crate::ftl::LoadedFtlFile>,
    locale_plans: Vec<SyncLocalePlan>,
}

fn build_sync_crate_plan(
    krate: &CrateInfo,
    target_locales: Option<&HashSet<String>>,
    create_missing: bool,
) -> Result<SyncCratePlan> {
    let ctx = LocaleContext::from_crate(krate, target_locales.is_none())?;
    if !ctx.assets_dir.is_dir() {
        bail!(
            "assets_dir for {} is missing or not a directory: {}",
            krate.name,
            ctx.assets_dir.display()
        );
    }

    let fallback_dir = ctx.locale_dir(&ctx.fallback);

    if !fallback_dir.is_dir() {
        bail!(
            "fallback locale directory '{}' is missing or not a directory for {}: {}; create the directory manually",
            ctx.fallback,
            krate.name,
            fallback_dir.display()
        );
    }

    // Discover all FTL files in the fallback locale (including namespaced ones)
    let fallback_files =
        CrateFtlLayout::from_assets_dir(&ctx.assets_dir, &ctx.fallback, &ctx.crate_name)
            .discover_and_load_files()?;

    let mut plans = Vec::new();
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
        let locale_path_exists = fs::symlink_metadata(&locale_dir).is_ok();
        if locale_path_exists && !crate::ftl::is_real_locale_directory(&locale_dir) {
            bail!(
                "target locale directory '{locale}' is not a directory for {}: {}",
                krate.name,
                locale_dir.display()
            );
        }

        let locale_created = !locale_path_exists;
        if !locale_path_exists && !create_missing {
            continue;
        }

        plans.push(SyncLocalePlan {
            locale: locale.to_string(),
            locale_dir,
            locale_created,
        });
    }

    preflight_sync_targets_parse(&fallback_files, &plans)?;

    Ok(SyncCratePlan {
        fallback_files,
        locale_plans: plans,
    })
}

pub(crate) fn preflight_sync_crate(
    krate: &CrateInfo,
    target_locales: Option<&HashSet<String>>,
    create_missing: bool,
) -> Result<()> {
    build_sync_crate_plan(krate, target_locales, create_missing).map(|_| ())
}

/// Sync all FTL files for a crate.
pub(crate) fn sync_crate(
    krate: &CrateInfo,
    target_locales: Option<&HashSet<String>>,
    dry_run: bool,
    create_missing: bool,
) -> Result<Vec<SyncLocaleResult>> {
    let SyncCratePlan {
        fallback_files,
        locale_plans,
    } = build_sync_crate_plan(krate, target_locales, create_missing)?;

    let mut results = Vec::new();
    for plan in locale_plans {
        if plan.locale_created && !dry_run {
            fs::create_dir_all(&plan.locale_dir)?;
        }

        if fallback_files.is_empty() {
            if plan.locale_created {
                results.push(SyncLocaleResult {
                    locale: plan.locale,
                    locale_created: true,
                    keys_added: 0,
                    added_keys: Vec::new(),
                    diff_info: None,
                });
            }
            continue;
        }

        // Sync each FTL file (main + namespaced)
        for (index, ftl_info) in fallback_files.iter().enumerate() {
            let mut result = sync_locale_file(
                &plan.locale_dir,
                &ftl_info.relative_path,
                &plan.locale,
                &ftl_info.resource,
                &ftl_info.keys,
                dry_run,
            )?;
            result.locale_created = plan.locale_created && index == 0;

            results.push(result);
        }
    }

    Ok(results)
}

fn preflight_sync_targets_parse(
    fallback_files: &[crate::ftl::LoadedFtlFile],
    plans: &[SyncLocalePlan],
) -> Result<()> {
    for plan in plans {
        for ftl_info in fallback_files {
            let ftl_file = plan.locale_dir.join(&ftl_info.relative_path);
            validate_sync_target_path(&plan.locale_dir, &ftl_file)?;
            if let Some(parent_dir) = ftl_file.parent()
                && parent_dir.exists()
                && !parent_dir.is_dir()
            {
                bail!(
                    "Refusing to sync '{}' because parent path '{}' is not a directory",
                    ftl_file.display(),
                    parent_dir.display()
                );
            }

            if ftl_file.exists() && !ftl_file.is_file() {
                bail!(
                    "Refusing to sync '{}' because target FTL path exists but is not a file",
                    ftl_file.display()
                );
            }

            if !ftl_file.exists() {
                continue;
            }

            let existing_content = fs::read_to_string(&ftl_file)?;
            let (_existing_resource, errors) =
                es_fluent_generate::ftl::parse_ftl_content(existing_content);
            if !errors.is_empty() {
                bail!(
                    "Refusing to sync '{}' because it contains Fluent parse errors: {}",
                    ftl_file.display(),
                    es_fluent_generate::ftl::format_parse_errors(&errors)
                );
            }
        }
    }

    Ok(())
}

fn validate_sync_target_path(locale_dir: &Path, ftl_file: &Path) -> Result<()> {
    let mut current = Some(ftl_file);
    while let Some(path) = current {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    if path == ftl_file {
                        bail!(
                            "Refusing to sync '{}' because target FTL paths must not be symlinks",
                            ftl_file.display()
                        );
                    }
                    bail!(
                        "Refusing to sync '{}' because target parent directories must not be symlinks: {}",
                        ftl_file.display(),
                        path.display()
                    );
                }
            },
            Err(error)
                if matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) => {},
            Err(error) => return Err(error.into()),
        }

        if path == locale_dir {
            break;
        }
        current = path.parent();
    }

    Ok(())
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
    validate_sync_target_path(locale_dir, &ftl_file)?;

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
            locale_created: false,
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
        locale_created: false,
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
            name: es_fluent_runner::PackageName::try_new("test-crate").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
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

    #[cfg(unix)]
    #[test]
    fn sync_locale_file_rejects_symlinked_target_ftl() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let locale_dir = temp.path().join("es");
        std::fs::create_dir_all(&locale_dir).expect("create locale dir");
        let outside_ftl = outside.path().join("test-crate.ftl");
        std::fs::write(&outside_ftl, "hello = Outside\n").expect("write outside ftl");
        std::os::unix::fs::symlink(&outside_ftl, locale_dir.join("test-crate.ftl"))
            .expect("create target ftl symlink");

        let fallback_resource = parse_resource("hello = Hello\nworld = World\n");
        let fallback_keys = crate::ftl::extract_message_keys(&fallback_resource);
        let err = sync_locale_file(
            &locale_dir,
            Path::new("test-crate.ftl"),
            "es",
            &fallback_resource,
            &fallback_keys,
            false,
        )
        .expect_err("symlinked target FTL should fail");

        assert!(err.to_string().contains("target FTL paths"));
        assert!(err.to_string().contains("symlinks"));
        let outside_content = std::fs::read_to_string(&outside_ftl).expect("read outside ftl");
        assert_eq!(outside_content, "hello = Outside\n");
    }

    #[cfg(unix)]
    #[test]
    fn sync_locale_file_rejects_symlinked_target_parent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let locale_dir = temp.path().join("es");
        std::fs::create_dir_all(&locale_dir).expect("create locale dir");
        std::fs::create_dir_all(outside.path().join("test-crate")).expect("create outside dir");
        let outside_ftl = outside.path().join("test-crate/ui.ftl");
        std::fs::write(&outside_ftl, "button = Outside\n").expect("write outside ftl");
        std::os::unix::fs::symlink(
            outside.path().join("test-crate"),
            locale_dir.join("test-crate"),
        )
        .expect("create target parent symlink");

        let fallback_resource = parse_resource("button = Button\nworld = World\n");
        let fallback_keys = crate::ftl::extract_message_keys(&fallback_resource);
        let err = sync_locale_file(
            &locale_dir,
            Path::new("test-crate/ui.ftl"),
            "es",
            &fallback_resource,
            &fallback_keys,
            false,
        )
        .expect_err("symlinked target parent should fail");

        assert!(err.to_string().contains("target parent directories"));
        assert!(err.to_string().contains("symlinks"));
        let outside_content = std::fs::read_to_string(&outside_ftl).expect("read outside ftl");
        assert_eq!(outside_content, "button = Outside\n");
    }

    #[test]
    fn sync_crate_errors_when_fallback_locale_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);
        std::fs::create_dir_all(temp.path().join("i18n/es")).expect("create non-fallback locale");

        let err = sync_crate(&krate, None, false, false)
            .expect_err("missing fallback locale directory should fail");

        assert!(err.to_string().contains("fallback locale directory"));
        assert!(err.to_string().contains("'en'"));
        assert!(err.to_string().contains("test-crate"));
    }

    #[test]
    fn sync_crate_errors_when_assets_dir_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);
        std::fs::write(temp.path().join("i18n"), "not a directory\n").expect("write assets file");

        let targets = HashSet::from(["fr-FR".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, true)
            .expect_err("assets_dir path as a file should fail");

        assert!(err.to_string().contains("assets_dir for test-crate"));
        assert!(err.to_string().contains("not a directory"));
        assert!(!temp.path().join("i18n/fr-FR").exists());
    }

    #[test]
    fn sync_crate_errors_when_fallback_locale_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);
        std::fs::create_dir_all(temp.path().join("i18n")).expect("create assets dir");
        std::fs::write(temp.path().join("i18n/en"), "not a directory\n")
            .expect("write fallback file");

        let targets = HashSet::from(["fr-FR".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, true)
            .expect_err("fallback locale path as a file should fail");

        assert!(err.to_string().contains("fallback locale directory"));
        assert!(err.to_string().contains("not a directory"));
        assert!(!temp.path().join("i18n/fr-FR").exists());
    }

    #[test]
    fn sync_crate_errors_when_target_locale_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);
        write_file(
            &temp.path().join("i18n/en/test-crate.ftl"),
            "hello = Hello\n",
        );
        std::fs::write(temp.path().join("i18n/fr-FR"), "not a directory\n")
            .expect("write target locale file");

        let targets = HashSet::from(["fr-FR".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, true)
            .expect_err("target locale path as a file should fail");

        assert!(err.to_string().contains("target locale directory"));
        assert!(err.to_string().contains("'fr-FR'"));
        assert!(err.to_string().contains("test-crate"));
        assert!(temp.path().join("i18n/fr-FR").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn sync_crate_errors_when_target_locale_path_is_symlink_without_fallback_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let krate = test_crate_with_i18n(&temp);
        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        std::fs::create_dir_all(outside.path().join("fr-FR")).expect("create outside locale");
        std::os::unix::fs::symlink(outside.path().join("fr-FR"), temp.path().join("i18n/fr-FR"))
            .expect("create target locale symlink");

        let targets = HashSet::from(["fr-FR".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, false)
            .expect_err("target locale symlink should fail before empty fallback succeeds");

        assert!(err.to_string().contains("target locale directory"));
        assert!(err.to_string().contains("'fr-FR'"));
        assert!(err.to_string().contains("test-crate"));
        assert!(temp.path().join("i18n/fr-FR").is_symlink());
        assert!(
            std::fs::read_dir(outside.path().join("fr-FR"))
                .expect("read outside locale")
                .next()
                .is_none(),
            "sync must not write through a symlinked target locale"
        );
    }

    #[test]
    fn sync_crate_create_makes_target_locale_when_fallback_has_no_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);
        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");

        let targets = HashSet::from(["fr-FR".to_string()]);
        let results = sync_crate(&krate, Some(&targets), false, true).expect("sync crate");

        assert!(
            temp.path().join("i18n/fr-FR").is_dir(),
            "create mode should create the requested locale directory even without FTL files"
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].locale, "fr-FR");
        assert!(results[0].locale_created);
        assert_eq!(results[0].keys_added, 0);
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

    #[test]
    fn sync_crate_preflights_target_parse_errors_before_writing_any_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);

        write_file(
            &temp.path().join("i18n/en/test-crate.ftl"),
            "hello = Hello\nworld = World\n",
        );
        write_file(
            &temp.path().join("i18n/en/test-crate/ui.ftl"),
            "button = Button\n",
        );

        let target_main = temp.path().join("i18n/es/test-crate.ftl");
        write_file(&target_main, "hello = Hola\n");
        write_file(
            &temp.path().join("i18n/es/test-crate/ui.ftl"),
            "broken = { $name\n",
        );
        let before = std::fs::read_to_string(&target_main).expect("read target before sync");

        let targets = HashSet::from(["es".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, false)
            .expect_err("target parse error should fail sync");

        assert!(err.to_string().contains("Refusing to sync"));
        assert!(err.to_string().contains("parse errors"));
        let after = std::fs::read_to_string(target_main).expect("read target after sync");
        assert_eq!(
            before, after,
            "sync should not write any target file after preflight parse errors"
        );
    }

    #[test]
    fn sync_crate_preflights_target_namespace_parent_file_before_writing_any_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);

        write_file(
            &temp.path().join("i18n/en/test-crate.ftl"),
            "hello = Hello\nworld = World\n",
        );
        write_file(
            &temp.path().join("i18n/en/test-crate/ui.ftl"),
            "button = Button\n",
        );

        let target_main = temp.path().join("i18n/es/test-crate.ftl");
        write_file(&target_main, "hello = Hola\n");
        std::fs::write(temp.path().join("i18n/es/test-crate"), "not a directory\n")
            .expect("write namespace blocker");
        let before = std::fs::read_to_string(&target_main).expect("read target before sync");

        let targets = HashSet::from(["es".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, false)
            .expect_err("target namespace parent file should fail sync");

        assert!(err.to_string().contains("Refusing to sync"));
        assert!(err.to_string().contains("parent path"));
        assert!(err.to_string().contains("not a directory"));
        let after = std::fs::read_to_string(target_main).expect("read target after sync");
        assert_eq!(
            before, after,
            "sync should not write any target file after target path preflight errors"
        );
    }

    #[test]
    fn sync_crate_preflights_target_ftl_directory_before_writing_any_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let krate = test_crate_with_i18n(&temp);

        write_file(
            &temp.path().join("i18n/en/test-crate.ftl"),
            "hello = Hello\nworld = World\n",
        );
        write_file(
            &temp.path().join("i18n/en/test-crate/ui.ftl"),
            "button = Button\n",
        );

        let target_main = temp.path().join("i18n/es/test-crate.ftl");
        write_file(&target_main, "hello = Hola\n");
        std::fs::create_dir_all(temp.path().join("i18n/es/test-crate/ui.ftl"))
            .expect("create target ftl directory");
        let before = std::fs::read_to_string(&target_main).expect("read target before sync");

        let targets = HashSet::from(["es".to_string()]);
        let err = sync_crate(&krate, Some(&targets), false, false)
            .expect_err("target ftl directory should fail sync");

        assert!(err.to_string().contains("Refusing to sync"));
        assert!(err.to_string().contains("target FTL path"));
        assert!(err.to_string().contains("not a file"));
        let after = std::fs::read_to_string(target_main).expect("read target after sync");
        assert_eq!(
            before, after,
            "sync should not write any target file after target file-path preflight errors"
        );
    }
}
