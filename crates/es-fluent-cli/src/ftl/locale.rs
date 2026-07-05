//! Locale context for iterating over locale directories.
//!
//! Provides a unified abstraction for the common pattern of iterating
//! over locale directories with `--all` flag support.

use crate::core::CrateInfo;
use anyhow::Result;
use es_fluent_toml::ResolvedI18nLayout;
use fs_err as fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LocalePathIssue {
    pub(crate) locale: String,
    pub(crate) path: PathBuf,
}

pub(crate) fn locale_named_non_directory_paths(
    assets_dir: &Path,
) -> std::io::Result<Vec<LocalePathIssue>> {
    let mut issues = Vec::new();

    for entry in fs::read_dir(assets_dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            continue;
        }

        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };

        match es_fluent_shared::parse_canonical_language_identifier(&name) {
            Ok(_)
            | Err(es_fluent_shared::CanonicalLanguageIdentifierError::NonCanonical { .. }) => {
                issues.push(LocalePathIssue { locale: name, path });
            },
            Err(
                es_fluent_shared::CanonicalLanguageIdentifierError::Invalid { .. }
                | es_fluent_shared::CanonicalLanguageIdentifierError::IcuInvalid { .. },
            ) => {},
        }
    }

    issues.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(issues)
}

pub(crate) fn is_real_locale_directory(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

/// Context for locale-based FTL file operations.
///
/// Encapsulates the common pattern of loading i18n config and iterating
/// over locale directories used by format, check, and sync commands.
#[derive(Clone, Debug)]
pub struct LocaleContext {
    /// The assets directory (e.g., `<crate>/i18n/`).
    pub assets_dir: PathBuf,
    /// The fallback language (e.g., "en").
    pub fallback: String,
    /// The locales to process.
    pub locales: Vec<String>,
    /// The crate name (for constructing FTL file paths).
    pub crate_name: String,
    /// Whether fallback-copy warnings are enabled by this crate's i18n.toml.
    pub check_fallback_copies: bool,
}

impl LocaleContext {
    /// Create a locale context from crate info.
    ///
    /// If `all` is true, includes all locale directories.
    /// Otherwise, includes only the fallback language.
    pub fn from_crate(krate: &CrateInfo, all: bool) -> Result<Self> {
        let layout =
            ResolvedI18nLayout::from_config_path(&krate.i18n_config_path).map_err(|error| {
                anyhow::anyhow!(
                    "Failed to read {}: {error}",
                    krate.i18n_config_path.display()
                )
            })?;
        let fallback = layout.fallback_language().to_string();

        let locales = if all {
            layout.available_locale_names()?
        } else {
            vec![fallback.clone()]
        };

        Ok(Self {
            assets_dir: layout.assets_dir,
            fallback,
            locales,
            crate_name: krate.name.to_string(),
            check_fallback_copies: layout.config.check_fallback_copies,
        })
    }

    /// Get the FTL file path for a specific locale.
    pub fn ftl_path(&self, locale: &str) -> PathBuf {
        self.assets_dir
            .join(locale)
            .join(format!("{}.ftl", self.crate_name))
    }

    /// Get the locale directory path.
    pub fn locale_dir(&self, locale: &str) -> PathBuf {
        self.assets_dir.join(locale)
    }

    /// Iterate over locales, yielding (locale, ftl_path) pairs.
    ///
    /// Only yields locales where the directory exists.
    #[cfg(test)]
    pub fn iter(&self) -> impl Iterator<Item = (&str, PathBuf)> {
        self.locales.iter().filter_map(|locale| {
            let locale_dir = self.locale_dir(locale);
            if locale_dir.exists() {
                Some((locale.as_str(), self.ftl_path(locale)))
            } else {
                None
            }
        })
    }

    /// Iterate over non-fallback locales.
    ///
    /// Useful for sync command which needs to skip the fallback.
    pub fn iter_non_fallback(&self) -> impl Iterator<Item = (&str, PathBuf)> {
        self.locales.iter().filter_map(|locale| {
            if locale == &self.fallback {
                return None;
            }
            let locale_dir = self.locale_dir(locale);
            if locale_dir.exists() {
                Some((locale.as_str(), self.ftl_path(locale)))
            } else {
                None
            }
        })
    }

    /// Check if a locale is the fallback language.
    #[cfg(test)]
    pub fn is_fallback(&self, locale: &str) -> bool {
        locale == self.fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_crate() -> (tempfile::TempDir, CrateInfo) {
        let temp_dir = tempfile::tempdir().unwrap();
        let assets = temp_dir.path().join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("en")).unwrap();
        fs::create_dir(assets.join("fr")).unwrap();
        fs::create_dir(assets.join("de")).unwrap();

        let config_path = temp_dir.path().join("i18n.toml");
        fs::write(
            &config_path,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();

        let krate = CrateInfo {
            name: es_fluent_runner::PackageName::try_new("test-crate").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(temp_dir.path().to_path_buf()),
            src_dir: crate::core::SourceDir::from_discovered(temp_dir.path().join("src")),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(config_path),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(assets.join("en")),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        (temp_dir, krate)
    }

    #[test]
    fn test_locale_context_fallback_only() {
        let (_temp, krate) = create_test_crate();
        let ctx = LocaleContext::from_crate(&krate, false).unwrap();

        assert_eq!(ctx.locales.len(), 1);
        assert_eq!(ctx.locales[0], "en");
        assert_eq!(ctx.fallback, "en");
    }

    #[test]
    fn test_locale_context_all_locales() {
        let (_temp, krate) = create_test_crate();
        let ctx = LocaleContext::from_crate(&krate, true).unwrap();

        assert_eq!(ctx.locales.len(), 3);
        assert!(ctx.locales.contains(&"en".to_string()));
        assert!(ctx.locales.contains(&"fr".to_string()));
        assert!(ctx.locales.contains(&"de".to_string()));
    }

    #[test]
    fn test_ftl_path() {
        let (_temp, krate) = create_test_crate();
        let ctx = LocaleContext::from_crate(&krate, false).unwrap();

        let path = ctx.ftl_path("en");
        assert!(path.ends_with("i18n/en/test-crate.ftl"));
    }

    #[test]
    fn test_iter_non_fallback() {
        let (_temp, krate) = create_test_crate();
        let ctx = LocaleContext::from_crate(&krate, true).unwrap();

        let non_fallback: Vec<_> = ctx.iter_non_fallback().map(|(l, _)| l).collect();
        assert!(!non_fallback.contains(&"en"));
        assert!(non_fallback.contains(&"fr"));
        assert!(non_fallback.contains(&"de"));
    }

    #[test]
    fn test_iter_and_is_fallback_cover_directory_presence() {
        let (temp, krate) = create_test_crate();
        // Remove one locale directory to ensure iter() skips missing dirs.
        std::fs::remove_dir_all(temp.path().join("i18n/fr")).unwrap();

        let ctx = LocaleContext::from_crate(&krate, true).unwrap();
        let locales_from_iter: Vec<_> = ctx.iter().map(|(l, _)| l.to_string()).collect();

        assert!(ctx.is_fallback("en"));
        assert!(!ctx.is_fallback("de"));
        assert!(locales_from_iter.contains(&"en".to_string()));
        assert!(!locales_from_iter.contains(&"fr".to_string()));
    }

    #[test]
    fn test_locale_context_rejects_noncanonical_locale_directory_names() {
        let temp_dir = tempfile::tempdir().unwrap();
        let assets = temp_dir.path().join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("en-us")).unwrap();
        fs::create_dir(assets.join("fr")).unwrap();

        let config_path = temp_dir.path().join("i18n.toml");
        fs::write(
            &config_path,
            "fallback_language = \"en-US\"\nassets_dir = \"i18n\"\n",
        )
        .unwrap();

        let krate = CrateInfo {
            name: es_fluent_runner::PackageName::try_new("test-crate").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(temp_dir.path().to_path_buf()),
            src_dir: crate::core::SourceDir::from_discovered(temp_dir.path().join("src")),
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(config_path),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(
                assets.join("en-US"),
            ),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        let err = LocaleContext::from_crate(&krate, true)
            .expect_err("noncanonical locale directories should fail");
        assert!(err.to_string().contains("en-us"));
        assert!(err.to_string().contains("en-US"));
    }

    #[test]
    fn locale_named_non_directory_paths_reports_only_locale_named_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let assets = temp_dir.path().join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("en")).unwrap();
        fs::write(assets.join("fr-FR"), "not a directory").unwrap();
        fs::write(assets.join("en-us"), "not a directory").unwrap();
        fs::write(assets.join("README.md"), "notes").unwrap();

        let issues = locale_named_non_directory_paths(&assets).expect("scan assets");

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].locale, "en-us");
        assert_eq!(issues[0].path, assets.join("en-us"));
        assert_eq!(issues[1].locale, "fr-FR");
        assert_eq!(issues[1].path, assets.join("fr-FR"));
    }

    #[cfg(unix)]
    #[test]
    fn locale_named_non_directory_paths_reports_locale_named_symlinked_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let assets = temp_dir.path().join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(outside.path().join("fr")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("fr"), assets.join("fr"))
            .expect("create locale symlink");

        let issues = locale_named_non_directory_paths(&assets).expect("scan assets");

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].locale, "fr");
        assert_eq!(issues[0].path, assets.join("fr"));
        assert!(!is_real_locale_directory(&assets.join("fr")));
    }
}
