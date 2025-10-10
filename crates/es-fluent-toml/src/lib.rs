use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{env, fs};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum I18nConfigError {
    /// Configuration file not found
    #[error("i18n.toml configuration file not found")]
    NotFound,
    /// Failed to read configuration file
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    /// Failed to parse configuration file
    #[error("Failed to parse configuration file: {0}")]
    ParseError(#[from] toml::de::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct I18nConfig {
    /// The fallback language identifier (e.g., "en")
    pub fallback_language: String,
    /// Path to the assets directory containing translation files
    /// Expected structure: {assets_dir}/{language}/{domain}.ftl
    pub assets_dir: PathBuf,
}

impl I18nConfig {
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self, I18nConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(I18nConfigError::NotFound);
        }

        let content = fs::read_to_string(path)?;

        let config: I18nConfig = toml::from_str(&content)?;

        Ok(config)
    }

    pub fn read_from_manifest_dir() -> Result<Self, I18nConfigError> {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| I18nConfigError::NotFound)?;

        let config_path = Path::new(&manifest_dir).join("i18n.toml");
        Self::read_from_path(config_path)
    }

    pub fn assets_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.assets_dir)
    }

    pub fn assets_dir_from_manifest(&self) -> Result<PathBuf, I18nConfigError> {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_| I18nConfigError::NotFound)?;

        Ok(Path::new(&manifest_dir).join(&self.assets_dir))
    }

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

    pub fn fallback_language_id(&self) -> &str {
        &self.fallback_language
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
            fallback_language: "en".to_string(),
            assets_dir: PathBuf::from("locales"),
        };

        assert_eq!(config.assets_dir_path(), PathBuf::from("locales"));
    }

    #[test]
    fn test_fallback_language_id() {
        let config = I18nConfig {
            fallback_language: "en".to_string(),
            assets_dir: PathBuf::from("i18n"),
        };

        assert_eq!(config.fallback_language_id(), "en");
    }
}
