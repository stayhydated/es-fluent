#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs, io};
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
    /// Encountered a language identifier that uses an unsupported subtag combination.
    #[error("Language identifier '{name}' is not supported: {reason}")]
    UnsupportedLanguageIdentifier {
        /// The invalid identifier.
        name: String,
        /// Explanation of why it is not supported.
        reason: String,
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
}

/// The configuration for `es-fluent`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct I18nConfig {
    /// The fallback language identifier (e.g., "en-US").
    pub fallback_language: String,
    /// Path to the assets directory containing translation files.
    /// Expected structure: {assets_dir}/{language}/{domain}.ftl
    pub assets_dir: PathBuf,
}

impl I18nConfig {
    /// Reads the configuration from a path.
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, I18nConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(I18nConfigError::NotFound);
        }

        let content = fs::read_to_string(path)?;

        let config: I18nConfig = toml::from_str(&content)?;

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
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| I18nConfigError::NotFound)?;

        Ok(Path::new(&manifest_dir).join(&self.assets_dir))
    }

    /// Returns the configured fallback language as a `LanguageIdentifier`.
    pub fn fallback_language_identifier(&self) -> Result<LanguageIdentifier, I18nConfigError> {
        let lang = self
            .fallback_language
            .parse::<LanguageIdentifier>()
            .map_err(
                |source| I18nConfigError::InvalidFallbackLanguageIdentifier {
                    name: self.fallback_language.clone(),
                    source,
                },
            )?;

        ensure_supported_language_identifier(&lang, &self.fallback_language)?;

        Ok(lang)
    }

    /// Returns the languages available under the assets directory.
    pub fn available_languages(&self) -> Result<Vec<LanguageIdentifier>, I18nConfigError> {
        let assets_path = self.assets_dir_from_manifest()?;
        let mut languages: Vec<(String, LanguageIdentifier)> = Vec::new();

        let entries = fs::read_dir(&assets_path).map_err(I18nConfigError::ReadError)?;

        for entry in entries {
            let entry = entry.map_err(I18nConfigError::ReadError)?;
            if !entry
                .file_type()
                .map_err(I18nConfigError::ReadError)?
                .is_dir()
            {
                continue;
            }

            let raw_name = entry.file_name();
            let name = raw_name.into_string().map_err(|raw| {
                I18nConfigError::ReadError(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Assets directory contains a non UTF-8 entry: {:?}", raw),
                ))
            })?;

            let lang = name.parse::<LanguageIdentifier>().map_err(|source| {
                I18nConfigError::InvalidLanguageIdentifier {
                    name: name.clone(),
                    source,
                }
            })?;

            ensure_supported_language_identifier(&lang, &name)?;

            languages.push((lang.to_string(), lang));
        }

        languages.sort_by(|a, b| a.0.cmp(&b.0));
        languages.dedup_by(|a, b| a.0 == b.0);

        Ok(languages.into_iter().map(|(_, lang)| lang).collect())
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
}

fn ensure_supported_language_identifier(
    lang: &LanguageIdentifier,
    original: &str,
) -> Result<(), I18nConfigError> {
    if lang.variants().next().is_some() {
        return Err(I18nConfigError::UnsupportedLanguageIdentifier {
            name: original.to_string(),
            reason: "variants are not supported".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};
    use tempfile::TempDir;

    #[test]
    fn test_read_from_path_success() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("i18n.toml");

        let config_content = r#"
fallback_language = "en"
assets_dir = "i18n"
"#;

        fs::write(&config_path, config_content).unwrap();

        let result = I18nConfig::read_from_path(&config_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.fallback_language, "en");
        assert_eq!(config.assets_dir, PathBuf::from("i18n"));
    }

    #[test]
    fn test_read_from_path_file_not_found() {
        let non_existent_path = Path::new("/non/existent/path/i18n.toml");
        let result = I18nConfig::read_from_path(non_existent_path);
        assert!(matches!(result, Err(I18nConfigError::NotFound)));
    }

    #[test]
    fn test_read_from_path_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("i18n.toml");

        let invalid_config = r#"
fallback_language = "en"
[invalid_section]
assets_dir = "i18n"
"#;

        fs::write(&config_path, invalid_config).unwrap();

        let result = I18nConfig::read_from_path(&config_path);
        assert!(matches!(result, Err(I18nConfigError::ParseError(_))));
    }

    #[test]
    fn test_assets_dir_path() {
        let config = I18nConfig {
            fallback_language: "en-US".to_string(),
            assets_dir: PathBuf::from("locales"),
        };

        assert_eq!(config.assets_dir_path(), PathBuf::from("locales"));
    }

    #[test]
    fn test_fallback_language_id() {
        let config = I18nConfig {
            fallback_language: "en-US".to_string(),
            assets_dir: PathBuf::from("i18n"),
        };

        assert_eq!(config.fallback_language_id(), "en-US");
    }

    #[test]
    fn test_fallback_language_identifier_success() {
        let config = I18nConfig {
            fallback_language: "en-US".to_string(),
            assets_dir: PathBuf::from("i18n"),
        };

        let lang = config.fallback_language_identifier().unwrap();

        assert_eq!(lang.to_string(), "en-US");
    }

    #[test]
    fn test_fallback_language_identifier_invalid() {
        let config = I18nConfig {
            fallback_language: "invalid-lang!".to_string(),
            assets_dir: PathBuf::from("i18n"),
        };

        let result = config.fallback_language_identifier();

        assert!(matches!(
            result,
            Err(I18nConfigError::InvalidFallbackLanguageIdentifier { name, .. })
                if name == "invalid-lang!"
        ));
    }

    #[test]
    fn test_available_languages_collects_directories() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_dir = temp_dir.path();
        let assets = manifest_dir.join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("en")).unwrap();
        fs::create_dir(assets.join("en-US")).unwrap();
        fs::create_dir(assets.join("fr")).unwrap();
        fs::create_dir(assets.join("zh-Hans")).unwrap();
        fs::write(assets.join("README.txt"), "ignored file").unwrap();

        unsafe { env::set_var("CARGO_MANIFEST_DIR", manifest_dir) };

        let config = I18nConfig {
            fallback_language: "en".to_string(),
            assets_dir: PathBuf::from("i18n"),
        };

        let languages = config.available_languages().unwrap();

        unsafe { env::remove_var("CARGO_MANIFEST_DIR") };

        let mut codes: Vec<String> = languages.into_iter().map(|lang| lang.to_string()).collect();
        codes.sort();

        assert_eq!(codes, vec!["en", "en-US", "fr", "zh-Hans"]);
    }

    #[test]
    fn test_available_languages_allows_language_only() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_dir = temp_dir.path();
        let assets = manifest_dir.join("i18n");
        fs::create_dir(&assets).unwrap();
        fs::create_dir(assets.join("en")).unwrap();

        unsafe { env::set_var("CARGO_MANIFEST_DIR", manifest_dir) };

        let config = I18nConfig {
            fallback_language: "en".to_string(),
            assets_dir: PathBuf::from("i18n"),
        };

        let languages = config.available_languages().unwrap();
        let codes: Vec<String> = languages.into_iter().map(|lang| lang.to_string()).collect();

        unsafe { env::remove_var("CARGO_MANIFEST_DIR") };

        assert_eq!(codes, vec!["en"]);
    }
}
