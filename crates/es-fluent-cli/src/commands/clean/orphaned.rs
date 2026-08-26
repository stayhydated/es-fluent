use super::super::common::WorkspaceCrates;
use crate::core::{CliError, CrateInfo};
use crate::ftl::LocaleContext;
use anstream::println;
use colored::Colorize as _;
use fs_err as fs;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct LocaleCleanupTarget {
    fallback_locale_dir: PathBuf,
    locale: String,
    locale_dir: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OrphanedFtlFile {
    pub(crate) abs_path: PathBuf,
    pub(crate) locale: String,
}

pub(super) struct OrphanedCleanupPlan {
    transaction: es_fluent_runner::FileTransaction,
    files: Vec<(PathBuf, String)>,
    files_checked: usize,
}

impl OrphanedCleanupPlan {
    pub(super) fn transaction(&self) -> &es_fluent_runner::FileTransaction {
        &self.transaction
    }
}

/// Get the expected FTL file paths for a locale by mirroring every relative
/// FTL path that exists in the fallback locale.
fn expected_files_for_locale(
    locale_dir: &Path,
    fallback_locale_dir: &Path,
) -> Result<HashSet<PathBuf>, CliError> {
    Ok(crate::ftl::discover_locale_ftl_files(fallback_locale_dir)?
        .into_iter()
        .map(|file| locale_dir.join(file.relative_path))
        .collect())
}

/// Clean orphaned FTL files that are absent from the fallback locale.
#[cfg(test)]
pub(super) fn clean_orphaned_files(
    workspace: &WorkspaceCrates,
    all_locales: bool,
    dry_run: bool,
) -> Result<(), CliError> {
    let plan = plan_orphaned_files(workspace, all_locales)?;
    if !dry_run {
        plan.transaction
            .commit()
            .map_err(|error| CliError::Other(error.to_string()))?;
    }
    render_orphaned_cleanup(&plan, dry_run);
    Ok(())
}

pub(super) fn plan_orphaned_files(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> Result<OrphanedCleanupPlan, CliError> {
    let mut transaction = es_fluent_runner::FileTransaction::default();
    let mut files = Vec::new();
    let mut files_checked = 0;
    let cleanup_targets = orphaned_scan_context(&workspace.crates, all_locales)?;

    for target in cleanup_targets {
        let expected_files =
            expected_files_for_locale(&target.locale_dir, &target.fallback_locale_dir)?;

        for file_info in crate::ftl::discover_locale_ftl_files(&target.locale_dir)? {
            files_checked += 1;

            if !expected_files.contains(&file_info.abs_path) {
                let display_path =
                    orphaned_display_path(workspace, &file_info.abs_path, &file_info.relative_path);
                transaction
                    .plan_remove(&file_info.abs_path)
                    .map_err(|error| CliError::Other(error.to_string()))?;
                if let Some(parent) = file_info.abs_path.parent() {
                    plan_empty_parent_dirs(&mut transaction, parent, &target.locale_dir);
                }
                files.push((file_info.abs_path, display_path));
            }
        }
    }

    Ok(OrphanedCleanupPlan {
        transaction,
        files,
        files_checked,
    })
}

pub(super) fn render_orphaned_cleanup(plan: &OrphanedCleanupPlan, dry_run: bool) {
    println!("{} Looking for orphaned FTL files...", "→".cyan());
    for (_, display_path) in &plan.files {
        if dry_run {
            println!(
                "{} Would remove orphaned file: {}",
                "•".yellow(),
                display_path.cyan()
            );
        } else {
            println!(
                "{} Removing orphaned file: {}",
                "✓".green(),
                display_path.cyan()
            );
        }
    }

    if plan.files.is_empty() {
        println!("\n{} No orphaned FTL files found.", "✓".green());
    } else if dry_run {
        println!(
            "\n{} Would remove {} orphaned file(s) (checked {} files)",
            "→".cyan(),
            plan.files.len().to_string().yellow(),
            plan.files_checked
        );
    } else {
        println!(
            "\n{} Removed {} orphaned file(s) (checked {} files)",
            "✓".green(),
            plan.files.len().to_string().cyan(),
            plan.files_checked
        );
    }
}

fn orphaned_display_path(
    workspace: &WorkspaceCrates,
    abs_path: &Path,
    fallback_relative_path: &Path,
) -> String {
    crate::utils::paths::slash_path(
        abs_path
            .strip_prefix(workspace.workspace_info.root_dir.as_path())
            .unwrap_or(fallback_relative_path),
    )
}

pub(super) fn validate_orphaned_scan_setup(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> Result<(), CliError> {
    orphaned_scan_context(&workspace.crates, all_locales)?;
    Ok(())
}

fn plan_empty_parent_dirs(
    transaction: &mut es_fluent_runner::FileTransaction,
    start: &std::path::Path,
    stop_at: &std::path::Path,
) {
    let mut current = start;

    while current != stop_at {
        transaction.plan_remove_empty_directory(current);

        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }
}

pub(crate) fn find_orphaned_files(
    workspace: &WorkspaceCrates,
    all_locales: bool,
) -> Result<Vec<PathBuf>, CliError> {
    Ok(find_orphaned_file_infos(&workspace.crates, all_locales)?
        .into_iter()
        .map(|file| file.abs_path)
        .collect())
}

pub(crate) fn find_orphaned_file_infos(
    scan_crates: &[CrateInfo],
    all_locales: bool,
) -> Result<Vec<OrphanedFtlFile>, CliError> {
    let cleanup_targets = orphaned_scan_context(scan_crates, all_locales)?;
    let mut orphaned = Vec::new();

    for target in cleanup_targets {
        let expected_files =
            expected_files_for_locale(&target.locale_dir, &target.fallback_locale_dir)?;

        for file_info in crate::ftl::discover_locale_ftl_files(&target.locale_dir)? {
            if !expected_files.contains(&file_info.abs_path) {
                orphaned.push(OrphanedFtlFile {
                    abs_path: file_info.abs_path,
                    locale: target.locale.clone(),
                });
            }
        }
    }

    orphaned.sort_by(|a, b| a.abs_path.cmp(&b.abs_path));
    Ok(orphaned)
}

fn orphaned_scan_context(
    scan_crates: &[CrateInfo],
    all_locales: bool,
) -> Result<BTreeSet<LocaleCleanupTarget>, CliError> {
    let mut cleanup_targets = BTreeSet::new();

    for krate in scan_crates {
        let ctx = LocaleContext::from_crate(krate, all_locales)
            .map_err(|e| CliError::from(std::io::Error::other(e)))?;
        let fallback_locale_dir = ctx.locale_dir(&ctx.fallback);
        validate_orphaned_fallback_locale_dir(
            &ctx.fallback,
            krate.name.as_str(),
            &fallback_locale_dir,
        )?;
        validate_orphaned_locale_ftl_paths(&fallback_locale_dir, krate.name.as_str())?;

        if all_locales {
            let mut invalid_paths = Vec::new();
            for issue in crate::ftl::locale_named_non_directory_paths(&ctx.assets_dir)? {
                invalid_paths.push(format!(
                    "{} for {}: {}",
                    issue.locale,
                    krate.name,
                    issue.path.display()
                ));
            }

            if !invalid_paths.is_empty() {
                invalid_paths.sort();
                return Err(CliError::Other(format!(
                    "locale path(s) are not directories: {}; refusing to scan orphaned files",
                    invalid_paths.join(", ")
                )));
            }
        }

        for (locale, _ftl_path) in ctx.iter_non_fallback() {
            let locale_dir = ctx.locale_dir(locale);
            validate_orphaned_locale_ftl_paths(&locale_dir, krate.name.as_str())?;
            cleanup_targets.insert(LocaleCleanupTarget {
                fallback_locale_dir: fallback_locale_dir.clone(),
                locale: locale.to_string(),
                locale_dir,
            });
        }
    }

    validate_expected_files_for_cleanup_targets(&cleanup_targets)?;

    Ok(cleanup_targets)
}

fn validate_orphaned_fallback_locale_dir(
    fallback: &str,
    crate_name: &str,
    fallback_locale_dir: &Path,
) -> Result<(), CliError> {
    let invalid_reason = match fs::symlink_metadata(fallback_locale_dir) {
        Ok(metadata) if metadata.file_type().is_symlink() => Some("is a symlink"),
        Ok(metadata) if metadata.is_dir() => None,
        Ok(_) => Some("is not a directory"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Some("is missing or not a directory")
        },
        Err(error) => {
            return Err(CliError::Other(format!(
                "fallback locale directory '{fallback}' could not be inspected for {crate_name}: {}: {error}; refusing to scan orphaned files",
                fallback_locale_dir.display()
            )));
        },
    };

    if let Some(reason) = invalid_reason {
        return Err(CliError::Other(format!(
            "fallback locale directory '{fallback}' {reason} for {crate_name}: {}; refusing to scan orphaned files because every non-fallback FTL file would look orphaned",
            fallback_locale_dir.display()
        )));
    }

    Ok(())
}

fn validate_expected_files_for_cleanup_targets(
    cleanup_targets: &BTreeSet<LocaleCleanupTarget>,
) -> Result<(), CliError> {
    for target in cleanup_targets {
        expected_files_for_locale(&target.locale_dir, &target.fallback_locale_dir).map_err(
            |error| {
                CliError::Other(format!(
                    "FTL file layout could not be read while preparing orphaned file expectations for locale '{}' at {}: {}; refusing to scan orphaned files",
                    target.locale,
                    target.locale_dir.display(),
                    error
                ))
            },
        )?;
    }

    Ok(())
}

fn validate_orphaned_locale_ftl_paths(
    locale_dir: &std::path::Path,
    crate_name: &str,
) -> Result<(), CliError> {
    crate::ftl::discover_locale_ftl_files(locale_dir).map_err(|error| {
        CliError::Other(format!(
            "FTL file layout could not be read for {crate_name}: {error}; refusing to scan orphaned files"
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::WorkspaceInfo;

    #[test]
    fn expected_files_include_nested_namespaced_paths() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let i18n_dir = temp.path().join("i18n");
        let fallback_dir = i18n_dir.join("en");
        let locale_dir = i18n_dir.join("es");

        std::fs::create_dir_all(fallback_dir.join("test-app-a/ui")).expect("create fallback dirs");
        std::fs::create_dir_all(&locale_dir).expect("create locale dir");

        // Main + nested namespaced file in fallback.
        std::fs::write(fallback_dir.join("test-app-a.ftl"), "hello = Hello\n")
            .expect("write fallback main");
        std::fs::write(
            fallback_dir.join("test-app-a/ui/button.ftl"),
            "button = Click\n",
        )
        .expect("write fallback namespaced");

        let expected =
            expected_files_for_locale(&locale_dir, &fallback_dir).expect("build expected files");

        assert!(expected.contains(&locale_dir.join("test-app-a.ftl")));
        assert!(expected.contains(&locale_dir.join("test-app-a/ui/button.ftl")));
    }

    #[test]
    fn expected_files_mirror_every_fallback_domain() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let i18n_dir = temp.path().join("i18n");
        let fallback_dir = i18n_dir.join("en");
        let locale_dir = i18n_dir.join("es");
        std::fs::create_dir_all(&fallback_dir).expect("create fallback");
        std::fs::create_dir_all(&locale_dir).expect("create locale");

        std::fs::write(fallback_dir.join("crate-a.ftl"), "a = A\n").expect("write fallback a");
        std::fs::write(fallback_dir.join("crate-b.ftl"), "b = B\n").expect("write fallback b");

        let expected =
            expected_files_for_locale(&locale_dir, &fallback_dir).expect("build expected files");

        assert!(expected.contains(&locale_dir.join("crate-a.ftl")));
        assert!(expected.contains(&locale_dir.join("crate-b.ftl")));
    }

    #[test]
    fn expected_files_are_empty_for_missing_fallback_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let files =
            expected_files_for_locale(&temp.path().join("locale"), &temp.path().join("missing"))
                .expect("missing dir should be ok");
        assert!(files.is_empty());
    }

    fn build_workspace(temp: &tempfile::TempDir) -> WorkspaceCrates {
        let manifest_dir = temp.path().to_path_buf();
        let src_dir = manifest_dir.join("src");
        std::fs::create_dir_all(&src_dir).expect("create src");
        std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
        let i18n_toml = manifest_dir.join("i18n.toml");
        std::fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        let krate = crate::core::CrateInfo {
            name: es_fluent_runner::PackageName::try_new("test-app").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: manifest_dir.clone(),
                target_dir: manifest_dir.join("target"),
                crates: vec![krate.clone()],
            },
            crates: vec![krate.clone()],
            valid: vec![krate],
            skipped: Vec::new(),
            package_not_found: None,
            all_i18n_package_names: vec!["test-app".to_string()],
        }
    }

    #[test]
    fn clean_orphaned_files_dry_run_preserves_orphans() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en");
        std::fs::create_dir_all(temp.path().join("i18n/es/test-app")).expect("create es");
        std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback");
        std::fs::write(temp.path().join("i18n/es/test-app.ftl"), "hello = Hola\n")
            .expect("write expected");
        std::fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
            .expect("write orphan");

        let result = clean_orphaned_files(&workspace, true, true);
        assert!(result.is_ok());
        assert!(temp.path().join("i18n/es/orphan.ftl").exists());
    }

    #[test]
    fn clean_orphaned_files_errors_when_fallback_locale_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
        std::fs::write(
            temp.path().join("i18n/fr/test-app.ftl"),
            "hello = Bonjour\n",
        )
        .expect("write non-fallback file");

        let result = clean_orphaned_files(&workspace, true, false)
            .expect_err("missing fallback locale should stop orphan cleanup");

        assert!(result.to_string().contains("fallback locale directory"));
        assert!(
            result
                .to_string()
                .contains("refusing to scan orphaned files")
        );
        assert!(
            temp.path().join("i18n/fr/test-app.ftl").exists(),
            "cleanup must not remove non-fallback files when fallback is missing"
        );
    }

    #[test]
    fn clean_orphaned_files_errors_when_fallback_locale_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
        std::fs::write(temp.path().join("i18n/en"), "not a directory\n")
            .expect("write fallback file");
        std::fs::write(
            temp.path().join("i18n/fr/test-app.ftl"),
            "hello = Bonjour\n",
        )
        .expect("write non-fallback file");

        let result = clean_orphaned_files(&workspace, true, false)
            .expect_err("fallback locale path as a file should stop orphan cleanup");

        assert!(result.to_string().contains("fallback locale directory"));
        assert!(result.to_string().contains("not a directory"));
        assert!(
            temp.path().join("i18n/fr/test-app.ftl").exists(),
            "cleanup must not remove non-fallback files when fallback path is a file"
        );
    }

    #[test]
    fn clean_orphaned_files_errors_when_locale_named_asset_path_is_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback");
        std::fs::write(temp.path().join("i18n/fr"), "not a directory\n")
            .expect("write locale file");

        let result = clean_orphaned_files(&workspace, true, true)
            .expect_err("locale-named file should stop orphan cleanup");

        assert!(result.to_string().contains("locale path"));
        assert!(result.to_string().contains("fr for test-app"));
        assert!(
            result
                .to_string()
                .contains("refusing to scan orphaned files")
        );
        assert!(
            temp.path().join("i18n/fr").is_file(),
            "cleanup must leave the locale-named file unchanged"
        );
    }

    #[test]
    fn clean_orphaned_files_errors_when_fallback_ftl_path_is_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en/test-app.ftl"))
            .expect("create ftl directory");
        std::fs::create_dir_all(temp.path().join("i18n/es")).expect("create target locale");
        std::fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
            .expect("write orphan");

        let result = clean_orphaned_files(&workspace, true, false)
            .expect_err("directory-valued fallback ftl path should stop orphan cleanup");

        assert!(
            result
                .to_string()
                .contains("Expected FTL path to be a file")
        );
        assert!(
            result
                .to_string()
                .contains("refusing to scan orphaned files"),
            "unexpected error: {result}"
        );
        assert!(
            temp.path().join("i18n/es/orphan.ftl").exists(),
            "cleanup must not remove orphaned files after FTL layout setup errors"
        );
    }

    #[test]
    fn clean_orphaned_files_errors_when_target_ftl_path_is_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback locale");
        std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback");
        std::fs::create_dir_all(temp.path().join("i18n/es/orphan.ftl"))
            .expect("create target ftl directory");

        let result = clean_orphaned_files(&workspace, true, false)
            .expect_err("directory-valued target ftl path should stop orphan cleanup");

        assert!(
            result
                .to_string()
                .contains("Expected FTL path to be a file")
        );
        assert!(
            result
                .to_string()
                .contains("refusing to scan orphaned files"),
            "unexpected error: {result}"
        );
        assert!(
            temp.path().join("i18n/es/orphan.ftl").is_dir(),
            "cleanup must leave directory-valued FTL paths unchanged"
        );
    }

    #[test]
    fn clean_orphaned_files_preflights_expected_files_before_removing_any_orphans() {
        let temp = tempfile::tempdir().expect("tempdir");

        let make_crate = |name: &str| {
            let manifest_dir = temp.path().join(name);
            let src_dir = manifest_dir.join("src");
            let i18n_toml = manifest_dir.join("i18n.toml");
            std::fs::create_dir_all(&src_dir).expect("create src");
            std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
            std::fs::write(
                &i18n_toml,
                "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
            )
            .expect("write i18n.toml");

            crate::core::CrateInfo {
                name: es_fluent_runner::PackageName::try_new(name).expect("valid package name"),
                manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
                src_dir: crate::core::SourceDir::from_discovered(src_dir),
                library_target_path: None,
                custom_build_target_path: None,
                i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
                ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                    manifest_dir.join("i18n/en"),
                ),
                has_lib_rs: true,
                fluent_features: Vec::new(),
            }
        };

        let crate_a = make_crate("crate-a");
        let crate_b = make_crate("crate-b");
        let workspace = WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: temp.path().to_path_buf(),
                target_dir: temp.path().join("target"),
                crates: vec![crate_a.clone(), crate_b.clone()],
            },
            crates: vec![crate_a.clone(), crate_b.clone()],
            valid: vec![crate_a, crate_b],
            skipped: Vec::new(),
            package_not_found: None,
            all_i18n_package_names: vec!["crate-a".to_string(), "crate-b".to_string()],
        };

        let crate_a_root = temp.path().join("crate-a");
        std::fs::create_dir_all(crate_a_root.join("i18n/en")).expect("create crate-a fallback");
        std::fs::create_dir_all(crate_a_root.join("i18n/es")).expect("create crate-a locale");
        std::fs::write(crate_a_root.join("i18n/en/crate-a.ftl"), "a = A\n")
            .expect("write crate-a fallback");
        let first_orphan = crate_a_root.join("i18n/es/orphan.ftl");
        std::fs::write(&first_orphan, "orphan = Orphan\n").expect("write first orphan");

        let crate_b_root = temp.path().join("crate-b");
        std::fs::create_dir_all(crate_b_root.join("i18n/en")).expect("create crate-b fallback");
        std::fs::create_dir_all(crate_b_root.join("i18n/es")).expect("create crate-b locale");
        std::fs::write(crate_b_root.join("i18n/en/crate-b.ftl"), "b = B\n")
            .expect("write crate-b fallback");
        std::fs::create_dir(crate_b_root.join("i18n/en/broken.ftl"))
            .expect("create malformed fallback FTL path");

        let result = clean_orphaned_files(&workspace, true, false)
            .expect_err("malformed expected fallback path should stop orphan cleanup");

        assert!(
            result
                .to_string()
                .contains("FTL file layout could not be read for crate-b"),
            "unexpected error: {result}"
        );
        assert!(
            result.to_string().contains("broken.ftl"),
            "unexpected error: {result}"
        );
        assert!(
            first_orphan.exists(),
            "cleanup must not remove earlier orphans after later expectation setup errors"
        );
    }

    #[test]
    fn find_orphaned_files_errors_when_fallback_locale_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/fr")).expect("create fr");
        std::fs::write(
            temp.path().join("i18n/fr/test-app.ftl"),
            "hello = Bonjour\n",
        )
        .expect("write non-fallback file");

        let result = find_orphaned_files(&workspace, true)
            .expect_err("missing fallback locale should stop orphan discovery");

        assert!(result.to_string().contains("fallback locale directory"));
        assert!(temp.path().join("i18n/fr/test-app.ftl").exists());
    }

    #[test]
    fn clean_orphaned_files_removes_orphans_in_real_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en/test-app/ui")).expect("create en");
        std::fs::create_dir_all(temp.path().join("i18n/es/test-app/ui")).expect("create es");
        std::fs::create_dir_all(temp.path().join("i18n/fr/test-app/ui")).expect("create fr");
        std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback");
        std::fs::write(
            temp.path().join("i18n/en/test-app/ui/button.ftl"),
            "button = Button\n",
        )
        .expect("write fallback namespace");
        std::fs::write(temp.path().join("i18n/es/test-app.ftl"), "hello = Hola\n")
            .expect("write expected");
        std::fs::write(
            temp.path().join("i18n/es/test-app/ui/button.ftl"),
            "button = Boton\n",
        )
        .expect("write expected namespace");
        std::fs::write(
            temp.path().join("i18n/fr/test-app.ftl"),
            "hello = Bonjour\n",
        )
        .expect("write fr expected");
        std::fs::write(
            temp.path().join("i18n/fr/test-app/ui/button.ftl"),
            "button = Bouton\n",
        )
        .expect("write fr expected namespace");
        std::fs::write(
            temp.path().join("i18n/es/test-app/ui/orphan.ftl"),
            "orphan = Orphan\n",
        )
        .expect("write orphan");
        std::fs::write(
            temp.path().join("i18n/fr/test-app/ui/orphan.ftl"),
            "orphan = Orphelin\n",
        )
        .expect("write fr orphan");

        let result = clean_orphaned_files(&workspace, true, false);
        assert!(result.is_ok());
        assert!(
            !temp.path().join("i18n/es/test-app/ui/orphan.ftl").exists(),
            "orphaned namespaced file should be removed"
        );
        assert!(
            !temp.path().join("i18n/fr/test-app/ui/orphan.ftl").exists(),
            "orphaned namespaced file should be removed from every non-fallback locale"
        );
        assert!(
            temp.path().join("i18n/es/test-app/ui/button.ftl").exists(),
            "expected namespaced file should remain"
        );
        assert!(
            temp.path().join("i18n/fr/test-app/ui/button.ftl").exists(),
            "expected namespaced file should remain in every non-fallback locale"
        );
    }

    #[test]
    fn clean_orphaned_files_removes_empty_nested_parent_dirs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en/test-app/ui")).expect("create en");
        std::fs::create_dir_all(temp.path().join("i18n/es/test-app/ui")).expect("create es");
        std::fs::create_dir_all(temp.path().join("i18n/es/test-app/stale/deep"))
            .expect("create stale dirs");
        std::fs::write(
            temp.path().join("i18n/en/test-app/ui/button.ftl"),
            "button = Button\n",
        )
        .expect("write fallback namespace");
        std::fs::write(
            temp.path().join("i18n/es/test-app/ui/button.ftl"),
            "button = Boton\n",
        )
        .expect("write expected namespace");
        std::fs::write(
            temp.path().join("i18n/es/test-app/stale/deep/orphan.ftl"),
            "orphan = Orphan\n",
        )
        .expect("write deep orphan");

        let result = clean_orphaned_files(&workspace, true, false);
        assert!(result.is_ok());
        assert!(!temp.path().join("i18n/es/test-app/stale").exists());
        assert!(
            temp.path().join("i18n/es/test-app/ui/button.ftl").exists(),
            "expected sibling namespace files should remain"
        );
        assert!(
            temp.path().join("i18n/es").is_dir(),
            "the locale directory itself should not be removed"
        );
    }

    #[test]
    fn clean_orphaned_files_keeps_expected_files_when_nothing_is_orphaned() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create en");
        std::fs::create_dir_all(temp.path().join("i18n/es")).expect("create es");
        std::fs::write(temp.path().join("i18n/en/test-app.ftl"), "hello = Hello\n")
            .expect("write fallback");
        std::fs::write(temp.path().join("i18n/es/test-app.ftl"), "hello = Hola\n")
            .expect("write expected");

        let result = clean_orphaned_files(&workspace, true, false);
        assert!(result.is_ok());
        assert!(temp.path().join("i18n/es/test-app.ftl").exists());
    }

    #[test]
    fn clean_orphaned_files_handles_shared_locale_roots_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_dir = temp.path().to_path_buf();
        let src_dir = manifest_dir.join("src");
        std::fs::create_dir_all(&src_dir).expect("create src");
        std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
        let i18n_toml = manifest_dir.join("i18n.toml");
        std::fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        let crate_a = crate::core::CrateInfo {
            name: es_fluent_runner::PackageName::try_new("crate-a").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir.clone()),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
                i18n_toml.clone(),
            ),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };
        let crate_b = crate::core::CrateInfo {
            name: es_fluent_runner::PackageName::try_new("crate-b").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };
        let workspace = WorkspaceCrates {
            workspace_info: WorkspaceInfo {
                root_dir: manifest_dir.clone(),
                target_dir: manifest_dir.join("target"),
                crates: vec![crate_a.clone(), crate_b.clone()],
            },
            crates: vec![crate_a.clone(), crate_b.clone()],
            valid: vec![crate_a, crate_b],
            skipped: Vec::new(),
            package_not_found: None,
            all_i18n_package_names: vec!["crate-a".to_string(), "crate-b".to_string()],
        };

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback");
        std::fs::create_dir_all(temp.path().join("i18n/es")).expect("create locale");
        std::fs::write(temp.path().join("i18n/en/crate-a.ftl"), "a = A\n").expect("write a");
        std::fs::write(temp.path().join("i18n/en/crate-b.ftl"), "b = B\n").expect("write b");
        std::fs::write(temp.path().join("i18n/es/crate-a.ftl"), "a = A\n").expect("write es a");
        std::fs::write(temp.path().join("i18n/es/crate-b.ftl"), "b = B\n").expect("write es b");
        std::fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
            .expect("write orphan");

        let result = clean_orphaned_files(&workspace, true, false);
        assert!(result.is_ok());
        assert!(temp.path().join("i18n/es/crate-a.ftl").exists());
        assert!(temp.path().join("i18n/es/crate-b.ftl").exists());
        assert!(!temp.path().join("i18n/es/orphan.ftl").exists());
    }

    #[test]
    fn orphan_scan_mirrors_fallback_paths_outside_package_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_dir = temp.path().to_path_buf();
        let src_dir = manifest_dir.join("src");
        std::fs::create_dir_all(&src_dir).expect("create src");
        std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
        let i18n_toml = manifest_dir.join("i18n.toml");
        std::fs::write(
            &i18n_toml,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        let crate_a = crate::core::CrateInfo {
            name: es_fluent_runner::PackageName::try_new("crate-a").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(manifest_dir.clone()),
            src_dir: crate::core::SourceDir::from_discovered(src_dir),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(i18n_toml),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                manifest_dir.join("i18n/en"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        std::fs::create_dir_all(temp.path().join("i18n/en")).expect("create fallback");
        std::fs::create_dir_all(temp.path().join("i18n/es")).expect("create locale");
        std::fs::write(temp.path().join("i18n/en/crate-a.ftl"), "a = A\n").expect("write a");
        std::fs::write(temp.path().join("i18n/en/crate-b.ftl"), "b = B\n").expect("write b");
        std::fs::write(temp.path().join("i18n/es/crate-a.ftl"), "a = A\n").expect("write es a");
        std::fs::write(temp.path().join("i18n/es/crate-b.ftl"), "b = B\n").expect("write es b");
        std::fs::write(temp.path().join("i18n/es/orphan.ftl"), "orphan = Orphan\n")
            .expect("write orphan");

        let selected_crates = [crate_a];
        let orphans =
            find_orphaned_file_infos(&selected_crates, true).expect("find selected crate orphans");
        assert_eq!(orphans.len(), 1);
        assert!(
            orphans[0].abs_path.ends_with("orphan.ftl"),
            "every fallback-relative FTL path should remain expected"
        );
    }
}
