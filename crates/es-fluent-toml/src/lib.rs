#![doc = include_str!("../README.md")]

pub mod build;
mod language;

use language::parse_language_entry;

use es_fluent_shared::{CanonicalLanguageIdentifierError, parse_canonical_language_identifier};
use fs_err::{self as fs, DirEntry};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;
use unic_langid::{LanguageIdentifier, LanguageIdentifierError};

#[derive(Debug, Error)]
pub enum I18nConfigError {
    /// Configuration file not found.
    #[error("i18n.toml configuration file not found")]
    NotFound,
    /// Failed to read configuration file.
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    /// Failed to parse configuration file.
    #[error("Failed to parse configuration file: {0}")]
    ParseError(#[from] toml::de::Error),
    /// Encountered an invalid language identifier while reading assets directory.
    #[error("Invalid language identifier '{name}' found in assets directory")]
    InvalidLanguageIdentifier {
        /// The invalid identifier.
        name: String,
        /// The parsing error produced by `unic-langid`.
        #[source]
        source: LanguageIdentifierError,
    },
    /// Encountered a non-canonical locale directory name.
    #[error("Locale directory '{name}' must use canonical BCP-47 casing '{canonical}'")]
    NonCanonicalLanguageIdentifier {
        /// The locale directory name found on disk.
        name: String,
        /// The canonical locale directory name expected by the runtime.
        canonical: String,
    },
    /// Encountered an invalid fallback language identifier.
    #[error("Invalid fallback language identifier '{name}'")]
    InvalidFallbackLanguageIdentifier {
        /// The invalid identifier.
        name: String,
        /// The parsing error produced by `unic-langid`.
        #[source]
        source: LanguageIdentifierError,
    },
    /// Encountered a non-canonical fallback language identifier.
    #[error("Fallback language '{name}' must use canonical BCP-47 casing '{canonical}'")]
    NonCanonicalFallbackLanguageIdentifier {
        /// The configured fallback language string.
        name: String,
        /// The canonical fallback language string expected by the runtime.
        canonical: String,
    },
}

/// Represents the `fluent_feature` field in `i18n.toml`.
/// Supports both a single string and an array of strings.
///
/// # Examples
///
/// Single feature:
/// ```toml
/// fluent_feature = "fluent"
/// ```
///
/// Multiple features:
/// ```toml
/// fluent_feature = ["fluent", "i18n"]
/// ```
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FluentFeature {
    /// A single feature name.
    Single(String),
    /// Multiple feature names.
    Multiple(Vec<String>),
}

impl FluentFeature {
    /// Returns the features as a vector of strings.
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            FluentFeature::Single(s) => vec![s.clone()],
            FluentFeature::Multiple(v) => v.clone(),
        }
    }

    /// Returns true if there are no features.
    pub fn is_empty(&self) -> bool {
        match self {
            FluentFeature::Single(s) => s.is_empty(),
            FluentFeature::Multiple(v) => v.is_empty(),
        }
    }
}

/// The configuration for `es-fluent`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct I18nConfig {
    /// The fallback language identifier (e.g., "en-US").
    pub fallback_language: String,
    /// Path to the assets directory containing translation files.
    /// Expected structure: {assets_dir}/{language}/{domain}.ftl
    pub assets_dir: PathBuf,
    /// Optional feature flag(s) that enable es-fluent derives in the crate.
    /// If specified, the CLI will enable these features when generating FTL files.
    ///
    /// # Examples
    ///
    /// Single feature:
    /// ```toml
    /// fluent_feature = "fluent"
    /// ```
    ///
    /// Multiple features:
    /// ```toml
    /// fluent_feature = ["fluent", "i18n"]
    /// ```
    #[serde(default)]
    pub fluent_feature: Option<FluentFeature>,
    /// Optional list of allowed namespaces for FTL file generation.
    /// If specified, only these namespace values can be used in `#[fluent(namespace = "...")]`.
    /// If not specified, any namespace is allowed.
    ///
    /// # Examples
    ///
    /// ```toml
    /// namespaces = ["ui", "errors", "messages"]
    /// ```
    #[serde(default)]
    pub namespaces: Option<Vec<String>>,
}

/// Fully resolved project i18n layout derived from `i18n.toml`.
#[derive(Clone, Debug)]
pub struct ResolvedI18nLayout {
    /// Manifest directory that owns the configuration.
    pub manifest_dir: PathBuf,
    /// Absolute path to `i18n.toml`.
    pub config_path: PathBuf,
    /// Parsed configuration.
    pub config: I18nConfig,
    /// Absolute path to the assets directory.
    pub assets_dir: PathBuf,
    /// Absolute path to the fallback locale output directory.
    pub output_dir: PathBuf,
}

impl ResolvedI18nLayout {
    /// Resolve layout from a manifest directory containing `i18n.toml`.
    pub fn from_manifest_dir(manifest_dir: &Path) -> Result<Self, I18nConfigError> {
        Self::from_config_path(manifest_dir.join("i18n.toml"))
    }

    /// Resolve layout from a concrete config path.
    pub fn from_config_path<P: AsRef<Path>>(config_path: P) -> Result<Self, I18nConfigError> {
        let config_path = config_path.as_ref();
        let manifest_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let config = I18nConfig::read_from_path(config_path)?;
        let assets_dir = config.assets_dir_from_base(Some(&manifest_dir))?;
        let output_dir = assets_dir.join(&config.fallback_language);

        Ok(Self {
            manifest_dir,
            config_path: config_path.to_path_buf(),
            config,
            assets_dir,
            output_dir,
        })
    }

    /// Returns the configured fallback locale string.
    pub fn fallback_language(&self) -> &str {
        &self.config.fallback_language
    }

    /// Returns the locale directory for `locale`.
    pub fn locale_dir(&self, locale: &str) -> PathBuf {
        self.assets_dir.join(locale)
    }

    /// Returns feature flags that enable derives for this crate.
    pub fn fluent_features(&self) -> Vec<String> {
        self.config
            .fluent_feature
            .as_ref()
            .map(FluentFeature::as_vec)
            .unwrap_or_default()
    }

    /// Returns available languages discovered from the assets directory.
    pub fn available_languages(&self) -> Result<Vec<LanguageIdentifier>, I18nConfigError> {
        self.config
            .available_languages_from_base(Some(&self.manifest_dir))
    }

    /// Returns available locale names discovered from the assets directory.
    pub fn available_locale_names(&self) -> Result<Vec<String>, I18nConfigError> {
        self.config
            .available_locale_names_from_base(Some(&self.manifest_dir))
    }

    /// Returns the configured namespace allowlist when present.
    pub fn allowed_namespaces(&self) -> Option<&[String]> {
        self.config.namespaces.as_deref()
    }
}

impl I18nConfig {
    /// Reads the configuration from a path.
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, I18nConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(I18nConfigError::NotFound);
        }

        let content = fs::read_to_string(path)?;

        let mut config: I18nConfig = toml::from_str(&content)?;
        config.fallback_language =
            parse_fallback_language_identifier(&config.fallback_language)?.to_string();

        Ok(config)
    }

    /// Reads the configuration from the manifest directory.
    pub fn read_from_manifest_dir() -> Result<Self, I18nConfigError> {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| I18nConfigError::NotFound)?;

        let config_path = Path::new(&manifest_dir).join("i18n.toml");
        Self::read_from_path(config_path)
    }

    /// Returns the path to the assets directory.
    pub fn assets_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.assets_dir)
    }

    /// Returns the path to the assets directory from the manifest directory.
    pub fn assets_dir_from_manifest(&self) -> Result<PathBuf, I18nConfigError> {
        self.assets_dir_from_base(None)
    }

    /// Returns the path to the assets directory from a base directory.
    /// If `base_dir` is `None`, uses `CARGO_MANIFEST_DIR` environment variable.
    pub fn assets_dir_from_base(
        &self,
        base_dir: Option<&Path>,
    ) -> Result<PathBuf, I18nConfigError> {
        let base = match base_dir {
            Some(dir) => dir.to_path_buf(),
            None => {
                let manifest_dir =
                    env::var("CARGO_MANIFEST_DIR").map_err(|_| I18nConfigError::NotFound)?;
                PathBuf::from(manifest_dir)
            },
        };

        Ok(base.join(&self.assets_dir))
    }

    /// Returns the configured fallback language as a `LanguageIdentifier`.
    pub fn fallback_language_identifier(&self) -> Result<LanguageIdentifier, I18nConfigError> {
        parse_fallback_language_identifier(&self.fallback_language)
    }

    /// Returns the languages available under the assets directory.
    pub fn available_languages(&self) -> Result<Vec<LanguageIdentifier>, I18nConfigError> {
        self.available_languages_from_base(None)
    }

    /// Returns the raw locale directory names under the assets directory.
    pub fn available_locale_names(&self) -> Result<Vec<String>, I18nConfigError> {
        self.available_locale_names_from_base(None)
    }

    /// Returns the languages available under the assets directory from a base directory.
    /// If `base_dir` is `None`, uses `CARGO_MANIFEST_DIR` environment variable.
    pub fn available_languages_from_base(
        &self,
        base_dir: Option<&Path>,
    ) -> Result<Vec<LanguageIdentifier>, I18nConfigError> {
        let assets_path = self.assets_dir_from_base(base_dir)?;
        let entries = fs::read_dir(&assets_path).map_err(I18nConfigError::ReadError)?;

        let mut languages: Vec<(String, LanguageIdentifier)> = collect_language_entries(entries)?
            .into_iter()
            .map(|entry| {
                let canonical = entry.language.to_string();
                (canonical, entry.language)
            })
            .collect();

        languages.sort_by(|a, b| a.0.cmp(&b.0));
        languages.dedup_by(|a, b| a.0 == b.0);

        Ok(languages.into_iter().map(|(_, lang)| lang).collect())
    }

    /// Returns the raw locale directory names under the assets directory from a base directory.
    /// If `base_dir` is `None`, uses `CARGO_MANIFEST_DIR` environment variable.
    pub fn available_locale_names_from_base(
        &self,
        base_dir: Option<&Path>,
    ) -> Result<Vec<String>, I18nConfigError> {
        let assets_path = self.assets_dir_from_base(base_dir)?;
        let entries = fs::read_dir(&assets_path).map_err(I18nConfigError::ReadError)?;

        let mut locales = collect_language_entries(entries)?
            .into_iter()
            .map(|entry| entry.raw_name)
            .collect::<Vec<_>>();

        locales.sort();
        Ok(locales)
    }

    /// Validates the assets directory.
    pub fn validate_assets_dir(&self) -> Result<(), I18nConfigError> {
        let assets_path = self.assets_dir_from_manifest()?;

        if !assets_path.exists() {
            return Err(I18nConfigError::ReadError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Assets directory '{}' does not exist",
                    assets_path.display()
                ),
            )));
        }

        if !assets_path.is_dir() {
            return Err(I18nConfigError::ReadError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Assets path '{}' is not a directory", assets_path.display()),
            )));
        }

        Ok(())
    }

    /// Returns the fallback language identifier.
    pub fn fallback_language_id(&self) -> &str {
        &self.fallback_language
    }

    /// Read configuration and resolve paths for a given manifest directory.
    ///
    /// This is a common pattern used across CLI tools and helpers.
    pub fn from_manifest_dir(manifest_dir: &Path) -> Result<Self, I18nConfigError> {
        let config_path = manifest_dir.join("i18n.toml");
        Self::read_from_path(config_path)
    }

    /// Get assets directory resolved from a manifest directory.
    pub fn assets_dir_from_manifest_dir(manifest_dir: &Path) -> Result<PathBuf, I18nConfigError> {
        let config = Self::from_manifest_dir(manifest_dir)?;
        config.assets_dir_from_base(Some(manifest_dir))
    }

    /// Get output directory (fallback language directory) from manifest directory.
    pub fn output_dir_from_manifest_dir(manifest_dir: &Path) -> Result<PathBuf, I18nConfigError> {
        let config = Self::from_manifest_dir(manifest_dir)?;
        let assets_dir = config.assets_dir_from_base(Some(manifest_dir))?;
        Ok(assets_dir.join(&config.fallback_language))
    }
}

fn parse_fallback_language_identifier(value: &str) -> Result<LanguageIdentifier, I18nConfigError> {
    parse_canonical_language_identifier(value).map_err(|err| match err {
        CanonicalLanguageIdentifierError::Invalid { source, .. } => {
            I18nConfigError::InvalidFallbackLanguageIdentifier {
                name: value.to_string(),
                source,
            }
        },
        CanonicalLanguageIdentifierError::NonCanonical { canonical, .. } => {
            I18nConfigError::NonCanonicalFallbackLanguageIdentifier {
                name: value.to_string(),
                canonical,
            }
        },
    })
}

fn collect_language_entries(
    entries: impl IntoIterator<Item = Result<DirEntry, std::io::Error>>,
) -> Result<Vec<language::ParsedLanguageEntry>, I18nConfigError> {
    let mut parsed_entries = Vec::new();

    for entry in entries {
        let entry = entry.map_err(I18nConfigError::ReadError)?;
        if let Some(entry) = parse_language_entry(entry)? {
            parsed_entries.push(entry);
        }
    }

    Ok(parsed_entries)
}

#[cfg(test)]
pub(crate) mod test_utils;

#[cfg(test)]
mod tests;
