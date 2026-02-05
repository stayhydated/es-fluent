//! Locale context for iterating over locale directories.
//!
//! Provides a unified abstraction for the common pattern of iterating
//! over locale directories with `--all` flag support.

use crate::core::CrateInfo;
use crate::utils::get_all_locales;
use anyhow::{Context as _, Result};
use es_fluent_toml::I18nConfig;
use std::collections::HashSet;
use std::path::PathBuf;

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
}

impl LocaleContext {
    /// Create a locale context from crate info.
    ///
    /// If `all` is true, includes all locale directories.
    /// Otherwise, includes only the fallback language.
    pub fn from_crate(krate: &CrateInfo, all: bool) -> Result<Self> {
        let config = I18nConfig::read_from_path(&krate.i18n_config_path)
            .with_context(|| format!("Failed to read {}", krate.i18n_config_path.display()))?;

        let assets_dir = krate.manifest_dir.join(&config.assets_dir);

        let locales = if all {
            get_all_locales(&assets_dir)?
        } else {
            vec![config.fallback_language.clone()]
        };

        Ok(Self {
            assets_dir,
            fallback: config.fallback_language,
            locales,
            crate_name: krate.name.clone(),
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
    pub fn is_fallback(&self, locale: &str) -> bool {
        locale == self.fallback
    }
}

/// Collect all available locales across all crates.
pub fn collect_all_available_locales(crates: &[CrateInfo]) -> Result<HashSet<String>> {
    let mut all_locales = HashSet::new();

    for krate in crates {
        let ctx = LocaleContext::from_crate(krate, true)?;
        for locale in ctx.locales {
            all_locales.insert(locale);
        }
    }

    Ok(all_locales)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn create_test_crate() -> (tempfile::TempDir, CrateInfo) {
        let temp_dir = tempdir().unwrap();
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
            name: "test-crate".to_string(),
            manifest_dir: temp_dir.path().to_path_buf(),
            src_dir: temp_dir.path().join("src"),
            i18n_config_path: config_path,
            ftl_output_dir: assets.join("en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        };

        (temp_dir, krate)
    }

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
}
