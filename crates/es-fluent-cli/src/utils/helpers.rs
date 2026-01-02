//! Common utility functions shared across CLI commands.

use crate::core::CrateInfo;
use crate::utils::ui;
use anyhow::{Context as _, Result};
use std::fs;
use std::path::Path;

/// Filter crates by package name if specified.
///
/// Returns all crates if `package` is `None`, otherwise returns only the crate
/// with the matching name. Prints a warning if the filter matches no crates.
pub fn filter_crates_by_package(
    crates: Vec<CrateInfo>,
    package: Option<&String>,
) -> Vec<CrateInfo> {
    match package {
        Some(pkg) => {
            let filtered: Vec<_> = crates.into_iter().filter(|c| &c.name == pkg).collect();
            if filtered.is_empty() {
                ui::print_package_not_found(pkg);
            }
            filtered
        },
        None => crates,
    }
}

/// Partition crates into valid (has lib.rs) and skipped (missing lib.rs).
///
/// Returns a tuple of (valid_crates, skipped_crates).
pub fn partition_by_lib_rs(crates: &[CrateInfo]) -> (Vec<&CrateInfo>, Vec<&CrateInfo>) {
    crates.iter().partition(|k| k.has_lib_rs)
}

/// Get all locale directories from an assets directory.
///
/// Returns a sorted list of locale directory names.
pub fn get_all_locales(assets_dir: &Path) -> Result<Vec<String>> {
    let mut locales = Vec::new();

    if !assets_dir.exists() {
        return Ok(locales);
    }

    for entry in fs::read_dir(assets_dir).context("Failed to read assets directory")? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            locales.push(name.to_string());
        }
    }

    locales.sort();
    Ok(locales)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_crate_info(name: &str, has_lib_rs: bool) -> CrateInfo {
        CrateInfo {
            name: name.to_string(),
            manifest_dir: PathBuf::new(),
            src_dir: PathBuf::new(),
            i18n_config_path: PathBuf::new(),
            ftl_output_dir: PathBuf::new(),
            has_lib_rs,
            fluent_features: Vec::new(),
        }
    }

    #[test]
    fn test_filter_crates_by_package_none() {
        let crates = vec![make_crate_info("foo", true), make_crate_info("bar", true)];
        let filtered = filter_crates_by_package(crates, None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_crates_by_package_some() {
        let crates = vec![make_crate_info("foo", true), make_crate_info("bar", true)];
        let pkg = "foo".to_string();
        let filtered = filter_crates_by_package(crates, Some(&pkg));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "foo");
    }

    #[test]
    fn test_partition_by_lib_rs() {
        let crates = vec![
            make_crate_info("with-lib", true),
            make_crate_info("without-lib", false),
        ];
        let (valid, skipped) = partition_by_lib_rs(&crates);
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].name, "with-lib");
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].name, "without-lib");
    }

    #[test]
    fn test_get_all_locales() {
        let temp_dir = tempfile::tempdir().unwrap();
        let assets = temp_dir.path();

        fs::create_dir(assets.join("en")).unwrap();
        fs::create_dir(assets.join("fr")).unwrap();
        fs::create_dir(assets.join("de")).unwrap();
        // Create a file to ensure it's filtered out
        fs::write(assets.join("README.md"), "test").unwrap();

        let locales = get_all_locales(assets).unwrap();
        assert_eq!(locales, vec!["de", "en", "fr"]);
    }

    #[test]
    fn test_get_all_locales_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let locales = get_all_locales(temp_dir.path()).unwrap();
        assert!(locales.is_empty());
    }

    #[test]
    fn test_get_all_locales_nonexistent() {
        let locales = get_all_locales(Path::new("/nonexistent/path")).unwrap();
        assert!(locales.is_empty());
    }
}
