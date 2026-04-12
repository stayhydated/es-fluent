use super::super::common::WorkspaceCrates;
use crate::core::CliError;
use crate::ftl::{CrateFtlLayout, LocaleContext, discover_locale_ftl_files};
use crate::utils::ui;
use colored::Colorize as _;
use std::collections::HashSet;
use std::path::PathBuf;

struct OrphanedCleaner<'a> {
    valid_crate_names: HashSet<&'a str>,
}

impl<'a> OrphanedCleaner<'a> {
    fn new(valid_crate_names: HashSet<&'a str>) -> Self {
        Self { valid_crate_names }
    }

    /// Get the expected FTL file paths for a crate based on registered types.
    /// This looks at what files the generate command would create.
    ///
    /// The logic:
    /// - A main FTL file (crate_name.ftl) is expected ONLY if it exists in the fallback locale
    /// - Namespaced files are expected if they exist in the fallback locale's crate subdirectory
    fn get_expected_ftl_files(
        &self,
        crate_name: &str,
        locale_dir: &std::path::Path,
        fallback_locale_dir: &std::path::Path,
    ) -> Result<HashSet<PathBuf>, CliError> {
        let mut expected = HashSet::new();
        let crate_names = std::iter::once(crate_name).chain(
            self.valid_crate_names
                .iter()
                .copied()
                .filter(|name| *name != crate_name),
        );

        for current_crate in crate_names {
            let fallback_layout =
                CrateFtlLayout::new(fallback_locale_dir.to_path_buf(), current_crate);
            let locale_layout = CrateFtlLayout::new(locale_dir.to_path_buf(), current_crate);
            expected.extend(locale_layout.expected_files_from_fallback(&fallback_layout)?);
        }

        Ok(expected)
    }

    #[cfg(test)]
    fn find_all_ftl_files(&self, dir: &std::path::Path) -> Result<Vec<PathBuf>, CliError> {
        Ok(discover_locale_ftl_files(dir)?
            .into_iter()
            .map(|info| info.abs_path)
            .collect())
    }
}

/// Clean orphaned FTL files that are no longer tied to any registered types.
pub(super) fn clean_orphaned_files(
    workspace: &WorkspaceCrates,
    all_locales: bool,
    dry_run: bool,
) -> Result<(), CliError> {
    ui::print_header();
    println!("{} Looking for orphaned FTL files...", "→".cyan());

    let mut total_removed = 0;
    let mut total_files_checked = 0;

    // Collect all valid crate names for quick lookup
    let valid_crate_names: HashSet<&str> =
        workspace.crates.iter().map(|c| c.name.as_str()).collect();
    let cleaner = OrphanedCleaner::new(valid_crate_names);

    // Track which files we've already processed to avoid duplicates
    // Use canonical paths to handle different ways of referring to the same file
    let mut processed_files: HashSet<std::path::PathBuf> = HashSet::new();

    // Also track which (locale_dir, relative_path) pairs we've seen
    let mut seen_paths: HashSet<(std::path::PathBuf, std::path::PathBuf)> = HashSet::new();

    for krate in &workspace.crates {
        let ctx = LocaleContext::from_crate(krate, all_locales)
            .map_err(|e| CliError::from(std::io::Error::other(e)))?;

        // Get the fallback locale directory for this crate
        let fallback_locale_dir = ctx.locale_dir(&ctx.fallback);

        for (locale, _ftl_path) in ctx.iter_non_fallback() {
            let locale_dir = ctx.locale_dir(locale);

            // Get the expected FTL files for this crate (based on what's in fallback)
            let expected_files =
                cleaner.get_expected_ftl_files(&krate.name, &locale_dir, &fallback_locale_dir)?;

            // Find all actual FTL files in the locale directory
            let actual_files = discover_locale_ftl_files(&locale_dir)?
                .into_iter()
                .map(|info| info.abs_path)
                .collect::<Vec<_>>();

            // Find orphaned files (actual files that are not in expected files)
            for file_path in actual_files {
                // Get canonical path for deduplication
                let canonical_path = file_path
                    .canonicalize()
                    .unwrap_or_else(|_| file_path.clone());

                // Skip if we've already processed this file
                if processed_files.contains(&canonical_path) {
                    continue;
                }
                processed_files.insert(canonical_path);

                total_files_checked += 1;
                let relative_path = file_path.strip_prefix(&locale_dir).unwrap_or(&file_path);

                // Create a unique key for this (locale_dir, relative_path) pair
                let path_key = (locale_dir.clone(), relative_path.to_path_buf());
                if seen_paths.contains(&path_key) {
                    continue;
                }
                seen_paths.insert(path_key);

                if !expected_files.contains(&file_path) {
                    total_removed += 1;

                    if dry_run {
                        println!(
                            "{} Would remove orphaned file: {}",
                            "•".yellow(),
                            relative_path.display().to_string().cyan()
                        );
                    } else {
                        println!(
                            "{} Removing orphaned file: {}",
                            "✓".green(),
                            relative_path.display().to_string().cyan()
                        );
                        std::fs::remove_file(&file_path)?;

                        // Try to remove empty parent directories
                        if let Some(parent) = file_path.parent()
                            && parent != locale_dir
                        {
                            let _ = std::fs::remove_dir(parent);
                        }
                    }
                }
            }
        }
    }

    if total_removed == 0 {
        println!("\n{} No orphaned FTL files found.", "✓".green());
    } else if dry_run {
        println!(
            "\n{} Would remove {} orphaned file(s) (checked {} files)",
            "→".cyan(),
            total_removed.to_string().yellow(),
            total_files_checked
        );
    } else {
        println!(
            "\n{} Removed {} orphaned file(s) (checked {} files)",
            "✓".green(),
            total_removed.to_string().cyan(),
            total_files_checked
        );
    }

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

        let valid_crates = HashSet::from(["test-app-a"]);
        let cleaner = OrphanedCleaner::new(valid_crates);
        let expected = cleaner
            .get_expected_ftl_files("test-app-a", &locale_dir, &fallback_dir)
            .expect("build expected files");

        assert!(expected.contains(&locale_dir.join("test-app-a.ftl")));
        assert!(expected.contains(&locale_dir.join("test-app-a/ui/button.ftl")));
    }

    #[test]
    fn find_all_ftl_files_discovers_nested_files() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let base = temp.path().join("nested");
        std::fs::create_dir_all(base.join("a/b")).expect("create nested dirs");
        std::fs::write(base.join("root.ftl"), "root = Root\n").expect("write root");
        std::fs::write(base.join("a/b/deep.ftl"), "deep = Deep\n").expect("write deep");
        std::fs::write(base.join("a/b/ignore.txt"), "noop").expect("write text");

        let cleaner = OrphanedCleaner::new(HashSet::new());
        let mut files = cleaner.find_all_ftl_files(&base).expect("discover files");
        files.sort();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&base.join("root.ftl")));
        assert!(files.contains(&base.join("a/b/deep.ftl")));
    }

    #[test]
    fn expected_files_include_other_valid_crates_with_fallback_files() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let i18n_dir = temp.path().join("i18n");
        let fallback_dir = i18n_dir.join("en");
        let locale_dir = i18n_dir.join("es");
        std::fs::create_dir_all(&fallback_dir).expect("create fallback");
        std::fs::create_dir_all(&locale_dir).expect("create locale");

        std::fs::write(fallback_dir.join("crate-a.ftl"), "a = A\n").expect("write fallback a");
        std::fs::write(fallback_dir.join("crate-b.ftl"), "b = B\n").expect("write fallback b");

        let valid_crates = HashSet::from(["crate-a", "crate-b"]);
        let cleaner = OrphanedCleaner::new(valid_crates);
        let expected = cleaner
            .get_expected_ftl_files("crate-a", &locale_dir, &fallback_dir)
            .expect("build expected files");

        assert!(expected.contains(&locale_dir.join("crate-a.ftl")));
        assert!(expected.contains(&locale_dir.join("crate-b.ftl")));
    }

    #[test]
    fn find_all_ftl_files_returns_empty_for_missing_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cleaner = OrphanedCleaner::new(HashSet::new());
        let missing = temp.path().join("missing");
        let files = cleaner
            .find_all_ftl_files(&missing)
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
            name: "test-app".to_string(),
            manifest_dir: manifest_dir.clone(),
            src_dir,
            i18n_config_path: i18n_toml,
            ftl_output_dir: manifest_dir.join("i18n/en"),
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
    fn clean_orphaned_files_removes_orphans_in_real_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = build_workspace(&temp);

        std::fs::create_dir_all(temp.path().join("i18n/en/test-app/ui")).expect("create en");
        std::fs::create_dir_all(temp.path().join("i18n/es/test-app/ui")).expect("create es");
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
            temp.path().join("i18n/es/test-app/ui/orphan.ftl"),
            "orphan = Orphan\n",
        )
        .expect("write orphan");

        let result = clean_orphaned_files(&workspace, true, false);
        assert!(result.is_ok());
        assert!(
            !temp.path().join("i18n/es/test-app/ui/orphan.ftl").exists(),
            "orphaned namespaced file should be removed"
        );
        assert!(
            temp.path().join("i18n/es/test-app/ui/button.ftl").exists(),
            "expected namespaced file should remain"
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
}
