use crate::commands::WorkspaceCrates;
use crate::core::CliError;
use crate::ftl::LocaleContext;
use crate::utils::{ftl::main_ftl_path, ui};
use colored::Colorize as _;
use std::collections::HashSet;

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
            let expected_files = get_expected_ftl_files(
                &krate.name,
                &locale_dir,
                &valid_crate_names,
                &fallback_locale_dir,
            );

            // Find all actual FTL files in the locale directory
            let actual_files = find_all_ftl_files(&locale_dir)?;

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

/// Get the expected FTL file paths for a crate based on registered types.
/// This looks at what files the generate command would create.
///
/// The logic:
/// - A main FTL file (crate_name.ftl) is expected ONLY if it exists in the fallback locale
/// - Namespaced files are expected if they exist in the fallback locale's crate subdirectory
fn get_expected_ftl_files(
    crate_name: &str,
    locale_dir: &std::path::Path,
    valid_crate_names: &HashSet<&str>,
    fallback_locale_dir: &std::path::Path,
) -> HashSet<std::path::PathBuf> {
    let mut expected = HashSet::new();

    // Extract locales from paths
    let fallback_locale = fallback_locale_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let locale = locale_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Check if main FTL file exists in fallback locale - if so, it's expected here too
    let fallback_main_ftl = main_ftl_path(
        fallback_locale_dir.parent().unwrap(),
        fallback_locale,
        crate_name,
    );
    if fallback_main_ftl.exists() {
        expected.insert(main_ftl_path(
            locale_dir.parent().unwrap(),
            locale,
            crate_name,
        ));
    }

    add_expected_namespaced_files_from_fallback(
        fallback_locale_dir,
        locale_dir,
        crate_name,
        &mut expected,
    );

    // Also add expected files for other crates (they're valid, not orphaned)
    for &other_crate in valid_crate_names.iter().filter(|&&c| c != crate_name) {
        // Extract fallback locale from fallback_locale_dir
        let fallback_locale = fallback_locale_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Extract current locale from locale_dir
        let locale = locale_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Only expect main FTL file if it exists in fallback
        let other_fallback_main = main_ftl_path(
            fallback_locale_dir.parent().unwrap(),
            fallback_locale,
            other_crate,
        );
        if other_fallback_main.exists() {
            expected.insert(main_ftl_path(
                locale_dir.parent().unwrap(),
                locale,
                other_crate,
            ));
        }

        add_expected_namespaced_files_from_fallback(
            fallback_locale_dir,
            locale_dir,
            other_crate,
            &mut expected,
        );
    }

    expected
}

/// Add expected namespaced files for `crate_name` by mirroring fallback locale paths.
///
/// Supports nested file-relative namespaces (e.g., `crate/ui/button.ftl`).
fn add_expected_namespaced_files_from_fallback(
    fallback_locale_dir: &std::path::Path,
    locale_dir: &std::path::Path,
    crate_name: &str,
    expected: &mut HashSet<std::path::PathBuf>,
) {
    let fallback_crate_subdir = fallback_locale_dir.join(crate_name);
    if !fallback_crate_subdir.exists() || !fallback_crate_subdir.is_dir() {
        return;
    }

    let locale_crate_subdir = locale_dir.join(crate_name);
    if let Ok(fallback_files) = find_all_ftl_files(&fallback_crate_subdir) {
        for fallback_path in fallback_files {
            if let Ok(relative) = fallback_path.strip_prefix(&fallback_crate_subdir) {
                expected.insert(locale_crate_subdir.join(relative));
            }
        }
    }
}

/// Recursively find all FTL files in a directory.
fn find_all_ftl_files(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, CliError> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            files.extend(find_all_ftl_files(&path)?);
        } else if path.extension().is_some_and(|e| e == "ftl") {
            files.push(path);
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let expected =
            get_expected_ftl_files("test-app-a", &locale_dir, &valid_crates, &fallback_dir);

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

        let mut files = find_all_ftl_files(&base).expect("discover files");
        files.sort();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&base.join("root.ftl")));
        assert!(files.contains(&base.join("a/b/deep.ftl")));
    }
}
