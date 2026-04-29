//! Common error types shared across the es-fluent ecosystem.

use std::path::PathBuf;
use thiserror::Error;
use unic_langid::LanguageIdentifier;

/// Common error types shared across the es-fluent ecosystem.
#[derive(Debug, Error)]
pub enum EsFluentError {
    /// Configuration file not found.
    #[error("Configuration file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    /// Failed to parse configuration file.
    #[error("Failed to parse configuration file: {0}")]
    ConfigParseError(#[from] toml::de::Error),

    /// Assets directory not found.
    #[error("Assets directory not found: {path}")]
    AssetsNotFound { path: PathBuf },

    /// Fallback language directory not found.
    #[error("Fallback language directory not found: {language}")]
    FallbackLanguageNotFound { language: String },

    /// Invalid language identifier.
    #[error("Invalid language identifier '{identifier}': {reason}")]
    InvalidLanguageIdentifier { identifier: String, reason: String },

    /// Language not supported.
    #[error("Language '{0}' is not supported")]
    LanguageNotSupported(LanguageIdentifier),

    /// Fluent parsing error.
    #[error("Fluent parsing error: {0:?}")]
    FluentParseError(Vec<fluent_syntax::parser::ParserError>),

    /// Fluent serialization error.
    #[error("Fluent serialization error: {0}")]
    FluentSerializeError(#[from] std::fmt::Error),

    /// IO error during file operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Environment variable error.
    #[error("Environment variable error: {0}")]
    EnvVarError(#[from] std::env::VarError),

    /// Generic backend error.
    #[error("Backend error: {0}")]
    BackendError(#[from] anyhow::Error),

    /// Missing package name.
    #[error("Missing package name")]
    MissingPackageName,
}

impl EsFluentError {
    /// Creates a configuration not found error.
    pub fn config_not_found(path: impl Into<PathBuf>) -> Self {
        Self::ConfigNotFound { path: path.into() }
    }

    /// Creates an assets not found error.
    pub fn assets_not_found(path: impl Into<PathBuf>) -> Self {
        Self::AssetsNotFound { path: path.into() }
    }

    /// Creates an invalid language identifier error.
    pub fn invalid_language_identifier(
        identifier: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidLanguageIdentifier {
            identifier: identifier.into(),
            reason: reason.into(),
        }
    }

    /// Creates a fallback language not found error.
    pub fn fallback_language_not_found(language: impl Into<String>) -> Self {
        Self::FallbackLanguageNotFound {
            language: language.into(),
        }
    }
}

/// A result type for common es-fluent operations.
pub type EsFluentResult<T> = Result<T, EsFluentError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn helper_constructors_build_expected_variants() {
        let config = EsFluentError::config_not_found("/tmp/i18n.toml");
        assert!(matches!(config, EsFluentError::ConfigNotFound { .. }));

        let assets = EsFluentError::assets_not_found("/tmp/i18n");
        assert!(matches!(assets, EsFluentError::AssetsNotFound { .. }));

        let invalid = EsFluentError::invalid_language_identifier("bad", "parse failure");
        assert!(matches!(
            invalid,
            EsFluentError::InvalidLanguageIdentifier { .. }
        ));

        let fallback = EsFluentError::fallback_language_not_found("en-US");
        assert!(matches!(
            fallback,
            EsFluentError::FallbackLanguageNotFound { .. }
        ));
    }

    #[test]
    fn display_messages_include_variant_context() {
        let supported = EsFluentError::LanguageNotSupported("fr".parse().unwrap());
        assert_eq!(supported.to_string(), "Language 'fr' is not supported");

        let fluent = EsFluentError::FluentParseError(Vec::new());
        assert_eq!(fluent.to_string(), "Fluent parsing error: []");

        let serialize = EsFluentError::FluentSerializeError(std::fmt::Error);
        assert_eq!(
            serialize.to_string(),
            "Fluent serialization error: an error occurred when formatting an argument"
        );

        let missing = EsFluentError::MissingPackageName;
        assert_eq!(missing.to_string(), "Missing package name");
    }

    #[test]
    fn source_errors_are_preserved_for_wrapped_variants() {
        let parse_error: EsFluentError = toml::from_str::<toml::Value>("=").unwrap_err().into();
        assert!(parse_error.source().is_some());

        let io_error: EsFluentError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing").into();
        assert!(io_error.source().is_some());

        let env_error: EsFluentError = std::env::VarError::NotPresent.into();
        assert!(env_error.source().is_some());

        let backend_error: EsFluentError = anyhow::anyhow!("backend").into();
        assert!(backend_error.source().is_some());
    }
}
