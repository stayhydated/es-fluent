#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), deny(clippy::panic, clippy::unwrap_used))]

mod language;

use es_fluent_shared::CanonicalLanguageIdentifierError;
use es_fluent_shared::namespace::{NamespacePathError, ResolvedNamespace};
use fs_err::{self as fs, DirEntry};
use path_slash::PathExt as _;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use unic_langid::{LanguageIdentifier, LanguageIdentifierError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LanguageEntryMode {
    Strict,
    CrateRootAssets,
}

const CRATE_ROOT_ASSET_IGNORED_DIRS: &[&str] = &[
    ".cargo", ".git", ".github", ".idea", ".vscode", "benches", "bin", "build", "dev", "dist",
    "doc", "docs", "examples", "lib", "man", "src", "target", "tests",
];

/// Directory names ignored as locale candidates when `assets_dir = "."`.
pub fn crate_root_asset_ignored_dir_names() -> &'static [&'static str] {
    CRATE_ROOT_ASSET_IGNORED_DIRS
}

impl LanguageEntryMode {
    fn should_ignore_dir_name(self, name: &str) -> bool {
        self == Self::CrateRootAssets && CRATE_ROOT_ASSET_IGNORED_DIRS.contains(&name)
    }

    fn should_ignore_error(self, error: &I18nConfigError) -> bool {
        match (self, error) {
            (
                Self::CrateRootAssets,
                I18nConfigError::InvalidLanguageIdentifier { .. }
                | I18nConfigError::IcuLanguageIdentifier { .. },
            ) => true,
            (
                Self::CrateRootAssets,
                I18nConfigError::NonCanonicalLanguageIdentifier { name, .. },
            ) if name == "src" => true,
            _ => false,
        }
    }
}

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
    /// Encountered a language identifier that could not be converted to ICU.
    #[error(
        "Language identifier '{name}' found in assets directory could not be parsed as an ICU locale: {details}"
    )]
    IcuLanguageIdentifier {
        /// The invalid identifier.
        name: String,
        /// The ICU parsing error.
        details: String,
    },
    /// Encountered a non-canonical locale directory name.
    #[error("Locale directory '{name}' must use canonical BCP-47 form '{canonical}'")]
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
    /// Encountered a fallback language identifier that could not be converted to ICU.
    #[error(
        "Fallback language identifier '{name}' could not be parsed as an ICU locale: {details}"
    )]
    IcuFallbackLanguageIdentifier {
        /// The invalid identifier.
        name: String,
        /// The ICU parsing error.
        details: String,
    },
    /// Encountered a non-canonical fallback language identifier.
    #[error("Fallback language '{name}' must use canonical BCP-47 form '{canonical}'")]
    NonCanonicalFallbackLanguageIdentifier {
        /// The configured fallback language string.
        name: String,
        /// The canonical fallback language string expected by the runtime.
        canonical: String,
    },
    /// Encountered an invalid configured namespace allowlist entry.
    #[error("Invalid namespace '{namespace}' in i18n.toml: {source}")]
    InvalidNamespace {
        /// The invalid namespace string.
        namespace: String,
        /// The namespace validation error.
        #[source]
        source: NamespacePathError,
    },
    /// Encountered an invalid configured assets directory.
    #[error("Invalid assets_dir '{path}' in i18n.toml: {reason}")]
    InvalidAssetsDir {
        /// The invalid assets_dir string.
        path: String,
        /// Explanation of the validation failure.
        reason: &'static str,
    },
}

/// Raw TOML shape for `i18n.toml` before validation and typed normalization.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RawI18nConfig {
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
    /// ```toml
    /// fluent_feature = ["fluent", "i18n"]
    /// ```
    #[serde(default)]
    pub fluent_feature: Option<Vec<String>>,
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
    /// Whether `cargo es-fluent check --all` should warn when a non-fallback
    /// locale copies the fallback message text.
    ///
    /// # Examples
    ///
    /// ```toml
    /// check_fallback_copies = false
    /// ```
    #[serde(default = "default_check_fallback_copies")]
    pub check_fallback_copies: bool,
}

impl RawI18nConfig {
    /// Validates raw TOML values and returns the typed configuration model.
    pub fn validate(self) -> Result<I18nConfig, I18nConfigError> {
        let fallback_language = parse_fallback_language_identifier(&self.fallback_language)?;
        let namespaces = self
            .namespaces
            .map(|namespaces| {
                namespaces
                    .into_iter()
                    .map(|namespace| {
                        ResolvedNamespace::new(namespace.clone()).map_err(|source| {
                            I18nConfigError::InvalidNamespace { namespace, source }
                        })
                    })
                    .collect()
            })
            .transpose()?;

        let assets_dir = normalize_relative_assets_dir(&self.assets_dir)?;

        Ok(I18nConfig {
            fallback_language,
            assets_dir,
            fluent_feature: self.fluent_feature,
            namespaces,
            check_fallback_copies: self.check_fallback_copies,
        })
    }
}

fn default_check_fallback_copies() -> bool {
    true
}

/// The configuration for `es-fluent`.
#[derive(bon::Builder, Clone, Debug)]
pub struct I18nConfig {
    /// The fallback language identifier (e.g., "en-US").
    pub fallback_language: LanguageIdentifier,
    /// Path to the assets directory containing translation files.
    /// Expected structure: {assets_dir}/{language}/{domain}.ftl
    #[builder(into)]
    pub assets_dir: PathBuf,
    /// Optional feature flag(s) that enable es-fluent derives in the crate.
    /// If specified, the CLI will enable these features when generating FTL files.
    ///
    /// # Examples
    ///
    /// ```toml
    /// fluent_feature = ["fluent", "i18n"]
    /// ```
    pub fluent_feature: Option<Vec<String>>,
    /// Optional list of allowed namespaces for FTL file generation.
    /// If specified, only these namespace values can be used in `#[fluent(namespace = "...")]`.
    /// If not specified, any namespace is allowed.
    ///
    /// # Examples
    ///
    /// ```toml
    /// namespaces = ["ui", "errors", "messages"]
    /// ```
    pub namespaces: Option<Vec<ResolvedNamespace>>,
    /// Whether `cargo es-fluent check --all` should warn when a non-fallback
    /// locale copies the fallback message text.
    #[builder(default = true)]
    pub check_fallback_copies: bool,
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
    /// Canonical fallback locale directory name.
    pub fallback_language: String,
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
        let fallback_language = config.fallback_language_id();
        let output_dir = assets_dir.join(&fallback_language);

        Ok(Self {
            manifest_dir,
            config_path: config_path.to_path_buf(),
            config,
            assets_dir,
            fallback_language,
            output_dir,
        })
    }

    /// Returns the configured fallback locale string.
    pub fn fallback_language(&self) -> &str {
        &self.fallback_language
    }

    /// Returns the locale directory for `locale`.
    pub fn locale_dir(&self, locale: &str) -> PathBuf {
        self.assets_dir.join(locale)
    }

    /// Returns feature flags that enable derives for this crate.
    pub fn fluent_features(&self) -> Vec<String> {
        self.config.fluent_feature.clone().unwrap_or_default()
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
    pub fn allowed_namespaces(&self) -> Option<&[ResolvedNamespace]> {
        self.config.namespaces.as_deref()
    }
}

impl I18nConfig {
    fn validate_resolved_assets_dir(assets_path: &Path) -> Result<(), I18nConfigError> {
        let display_path = assets_path.to_slash_lossy();

        if !assets_path.exists() {
            return Err(I18nConfigError::ReadError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Assets directory '{display_path}' does not exist"),
            )));
        }

        if !assets_path.is_dir() {
            return Err(I18nConfigError::ReadError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Assets path '{display_path}' is not a directory"),
            )));
        }

        Ok(())
    }

    fn validated_assets_dir_from_base(
        &self,
        base_dir: Option<&Path>,
    ) -> Result<PathBuf, I18nConfigError> {
        let assets_path = self.assets_dir_from_base(base_dir)?;
        Self::validate_resolved_assets_dir(&assets_path)?;
        Ok(assets_path)
    }

    /// Reads the configuration from a path.
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, I18nConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(I18nConfigError::NotFound);
        }

        let content = fs::read_to_string(path)?;

        let raw: RawI18nConfig = toml::from_str(&content)?;
        raw.validate()
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
        let assets_dir = normalize_relative_assets_dir(&self.assets_dir)?;
        let base = match base_dir {
            Some(dir) => dir.to_path_buf(),
            None => {
                let manifest_dir =
                    env::var("CARGO_MANIFEST_DIR").map_err(|_| I18nConfigError::NotFound)?;
                PathBuf::from(manifest_dir)
            },
        };

        let assets_path = base.join(&assets_dir);
        validate_existing_components_stay_inside_base(
            &assets_path,
            &base,
            &self.assets_dir.to_slash_lossy(),
        )?;
        validate_existing_assets_dir_components_are_real(
            &base,
            &assets_dir,
            &self.assets_dir.to_slash_lossy(),
        )?;
        Ok(assets_path)
    }

    /// Returns the configured fallback language as a `LanguageIdentifier`.
    pub fn fallback_language_identifier(&self) -> Result<LanguageIdentifier, I18nConfigError> {
        Ok(self.fallback_language.clone())
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
        let assets_path = self.validated_assets_dir_from_base(base_dir)?;
        let entries = fs::read_dir(&assets_path).map_err(I18nConfigError::ReadError)?;
        let entry_mode = self.language_entry_mode()?;

        let mut languages: Vec<(String, LanguageIdentifier)> =
            collect_language_entries(entries, entry_mode)?
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
        let assets_path = self.validated_assets_dir_from_base(base_dir)?;
        let entries = fs::read_dir(&assets_path).map_err(I18nConfigError::ReadError)?;
        let entry_mode = self.language_entry_mode()?;

        let mut locales = collect_language_entries(entries, entry_mode)?
            .into_iter()
            .map(|entry| entry.raw_name)
            .collect::<Vec<_>>();

        locales.sort();
        Ok(locales)
    }

    fn language_entry_mode(&self) -> Result<LanguageEntryMode, I18nConfigError> {
        let assets_dir = normalize_relative_assets_dir(&self.assets_dir)?;
        if assets_dir == Path::new(".") {
            Ok(LanguageEntryMode::CrateRootAssets)
        } else {
            Ok(LanguageEntryMode::Strict)
        }
    }

    /// Validates the assets directory.
    pub fn validate_assets_dir(&self) -> Result<(), I18nConfigError> {
        let assets_path = self.assets_dir_from_manifest()?;
        Self::validate_resolved_assets_dir(&assets_path)
    }

    /// Returns the fallback language identifier.
    pub fn fallback_language_id(&self) -> String {
        self.fallback_language.to_string()
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
        Ok(assets_dir.join(config.fallback_language_id()))
    }
}

fn normalize_relative_assets_dir(path: &Path) -> Result<PathBuf, I18nConfigError> {
    if path.as_os_str().is_empty() {
        return Err(I18nConfigError::InvalidAssetsDir {
            path: path.to_slash_lossy().to_string(),
            reason: "must point to a locale asset directory",
        });
    }
    if path.is_absolute() {
        return Err(I18nConfigError::InvalidAssetsDir {
            path: path.to_slash_lossy().to_string(),
            reason: "must be relative to the crate root",
        });
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {},
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(I18nConfigError::InvalidAssetsDir {
                        path: path.to_slash_lossy().to_string(),
                        reason: "must stay inside the crate root",
                    });
                }
            },
            Component::Prefix(_) | Component::RootDir => {
                return Err(I18nConfigError::InvalidAssetsDir {
                    path: path.to_slash_lossy().to_string(),
                    reason: "must be relative to the crate root",
                });
            },
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }

    Ok(normalized)
}

fn validate_existing_components_stay_inside_base(
    path: &Path,
    base: &Path,
    raw_path: &str,
) -> Result<(), I18nConfigError> {
    let Ok(base) = base.canonicalize() else {
        return Ok(());
    };

    let Some(existing_ancestor) = path.ancestors().find(|ancestor| ancestor.exists()) else {
        return Ok(());
    };
    let Ok(existing_ancestor) = existing_ancestor.canonicalize() else {
        return Ok(());
    };

    if existing_ancestor.starts_with(&base) {
        return Ok(());
    }

    Err(I18nConfigError::InvalidAssetsDir {
        path: raw_path.to_string(),
        reason: "must stay inside the crate root after resolving existing path components",
    })
}

fn validate_existing_assets_dir_components_are_real(
    base: &Path,
    assets_dir: &Path,
    raw_path: &str,
) -> Result<(), I18nConfigError> {
    let mut current = base.to_path_buf();

    for component in assets_dir.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(I18nConfigError::InvalidAssetsDir {
                    path: raw_path.to_string(),
                    reason: "existing path components must be real directories, not symlinks",
                });
            },
            Ok(_) => {},
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return Ok(());
            },
            Err(_) => return Ok(()),
        }
    }

    Ok(())
}

fn parse_fallback_language_identifier(value: &str) -> Result<LanguageIdentifier, I18nConfigError> {
    es_fluent_shared::parse_canonical_language_identifier(value).map_err(|err| match err {
        CanonicalLanguageIdentifierError::Invalid { source, .. } => {
            I18nConfigError::InvalidFallbackLanguageIdentifier {
                name: value.to_string(),
                source,
            }
        },
        CanonicalLanguageIdentifierError::IcuInvalid { details, .. } => {
            I18nConfigError::IcuFallbackLanguageIdentifier {
                name: value.to_string(),
                details,
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
    mode: LanguageEntryMode,
) -> Result<Vec<language::ParsedLanguageEntry>, I18nConfigError> {
    let mut parsed_entries = Vec::new();

    for entry in entries {
        let entry = entry.map_err(I18nConfigError::ReadError)?;
        if entry
            .file_type()
            .map_err(I18nConfigError::ReadError)?
            .is_dir()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|name| mode.should_ignore_dir_name(name))
        {
            continue;
        }

        match language::parse_language_entry(entry) {
            Ok(Some(entry)) => parsed_entries.push(entry),
            Ok(None) => {},
            Err(error) if mode.should_ignore_error(&error) => {},
            Err(error) => return Err(error),
        }
    }

    Ok(parsed_entries)
}

#[cfg(test)]
pub(crate) mod test_utils;

#[cfg(test)]
mod tests;
