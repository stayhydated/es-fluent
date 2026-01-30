//! This module provides the error types for `es-fluent-derive-core`.

use proc_macro_error2::{abort, abort_call_site, emit_error};
use proc_macro2::Span;
use std::path::PathBuf;
use thiserror::Error;
use unic_langid::LanguageIdentifier;

/// An error that can occur when parsing `es-fluent` attributes.
#[derive(Clone, Debug, thiserror::Error)]
pub enum EsFluentCoreError {
    /// An error related to Fluent attribute parsing.
    #[error("Attribute error: {message}")]
    AttributeError { message: String, span: Option<Span> },

    /// An error related to variant consistency.
    #[error("Variant '{variant_name}' error: {message}")]
    VariantError {
        message: String,
        variant_name: String,
        span: Option<Span>,
    },

    /// An error related to field processing.
    #[error("{}", field_error_fmt(.message, .field_name))]
    FieldError {
        message: String,
        field_name: Option<String>,
        span: Option<Span>,
    },

    /// An error with conversions or transformations.
    #[error("Transform error: {message}")]
    TransformError { message: String, span: Option<Span> },
}

fn field_error_fmt(message: &str, field_name: &Option<String>) -> String {
    match field_name {
        Some(name) => format!("Field '{}' error: {}", name, message),
        None => format!("Field error: {}", message),
    }
}

impl EsFluentCoreError {
    /// Returns the span of the error.
    pub fn span(&self) -> Option<Span> {
        match self {
            EsFluentCoreError::AttributeError { span, .. } => *span,
            EsFluentCoreError::VariantError { span, .. } => *span,
            EsFluentCoreError::FieldError { span, .. } => *span,
            EsFluentCoreError::TransformError { span, .. } => *span,
        }
    }

    /// Returns a mutable reference to the error message.
    pub fn message_mut(&mut self) -> &mut String {
        match self {
            EsFluentCoreError::AttributeError { message, .. } => message,
            EsFluentCoreError::VariantError { message, .. } => message,
            EsFluentCoreError::FieldError { message, .. } => message,
            EsFluentCoreError::TransformError { message, .. } => message,
        }
    }

    /// Aborts the macro execution with the error.
    pub fn abort(self) -> ! {
        let msg = self.to_string();
        match self.span() {
            Some(span) => abort!(span, "{}", msg),
            None => abort_call_site!("{}", msg),
        }
    }

    /// Emits the error as a compiler error.
    pub fn emit(&self) {
        let msg = self.to_string();
        match self.span() {
            Some(span) => emit_error!(span, "{}", msg),
            None => emit_error!("{}", msg),
        }
    }
}

/// A trait for adding notes and help messages to an error.
pub trait ErrorExt {
    /// Adds a note to the error.
    fn with_note(self, note: String) -> Self;
    /// Adds a help message to the error.
    fn with_help(self, help: String) -> Self;
}

impl ErrorExt for EsFluentCoreError {
    fn with_note(mut self, note_msg: String) -> Self {
        let message = self.message_mut();
        *message = format!("{}\nnote: {}", message, note_msg);
        self
    }

    fn with_help(mut self, help_msg: String) -> Self {
        let message = self.message_mut();
        *message = format!("{}\nhelp: {}", message, help_msg);
        self
    }
}

/// A result type for `es-fluent-derive-core`.
pub type EsFluentCoreResult<T> = Result<T, EsFluentCoreError>;

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

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

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
