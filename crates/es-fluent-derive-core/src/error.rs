//! This module provides the error types for `es-fluent-derive-core`.

use proc_macro2::Span;
use std::fmt;

pub use es_fluent_shared::error::{EsFluentError, EsFluentResult};

/// Attribute parsing context used for diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttrContext {
    MessageContainer,
    MessageStructContainer,
    MessageEnumContainer,
    MessageField,
    EnumVariant,
    VariantsContainer,
    VariantsField,
    VariantsVariant,
    LabelContainer,
    ChoiceContainer,
    LanguageContainer,
}

impl fmt::Display for AttrContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::MessageContainer => "message container",
            Self::MessageStructContainer => "message struct container",
            Self::MessageEnumContainer => "message enum container",
            Self::MessageField => "message field",
            Self::EnumVariant => "enum variant",
            Self::VariantsContainer => "variants container",
            Self::VariantsField => "variants field",
            Self::VariantsVariant => "variants variant",
            Self::LabelContainer => "label container",
            Self::ChoiceContainer => "choice container",
            Self::LanguageContainer => "language container",
        };
        f.write_str(label)
    }
}

/// Structured attribute diagnostic data.
#[derive(Clone, Debug)]
pub struct AttrError {
    pub context: AttrContext,
    pub message: String,
    pub span: Option<Span>,
    pub note: Option<String>,
    pub help: Option<String>,
}

impl AttrError {
    pub fn new(context: AttrContext, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            context,
            message: message.into(),
            span,
            note: None,
            help: None,
        }
    }

    pub fn message_mut(&mut self) -> &mut String {
        &mut self.message
    }
}

impl fmt::Display for AttrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Attribute error in {}: {}", self.context, self.message)?;
        if let Some(note) = &self.note {
            write!(f, "\nnote: {note}")?;
        }
        if let Some(help) = &self.help {
            write!(f, "\nhelp: {help}")?;
        }
        Ok(())
    }
}

/// An error that can occur when parsing `es-fluent` attributes.
#[derive(Clone, Debug, thiserror::Error)]
pub enum EsFluentCoreError {
    /// An error related to Fluent attribute parsing.
    #[error("Attribute error: {message}")]
    AttributeError { message: String, span: Option<Span> },

    /// A structured error related to Fluent attribute parsing.
    #[error("{0}")]
    StructuredAttributeError(AttrError),

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
            EsFluentCoreError::StructuredAttributeError(error) => error.span,
            EsFluentCoreError::VariantError { span, .. } => *span,
            EsFluentCoreError::FieldError { span, .. } => *span,
            EsFluentCoreError::TransformError { span, .. } => *span,
        }
    }

    /// Returns a mutable reference to the error message.
    pub fn message_mut(&mut self) -> &mut String {
        match self {
            EsFluentCoreError::AttributeError { message, .. } => message,
            EsFluentCoreError::StructuredAttributeError(error) => error.message_mut(),
            EsFluentCoreError::VariantError { message, .. } => message,
            EsFluentCoreError::FieldError { message, .. } => message,
            EsFluentCoreError::TransformError { message, .. } => message,
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
        if let EsFluentCoreError::StructuredAttributeError(error) = &mut self {
            error.note = Some(note_msg);
            return self;
        }

        let message = self.message_mut();
        *message = format!("{}\nnote: {}", message, note_msg);
        self
    }

    fn with_help(mut self, help_msg: String) -> Self {
        if let EsFluentCoreError::StructuredAttributeError(error) = &mut self {
            error.help = Some(help_msg);
            return self;
        }

        let message = self.message_mut();
        *message = format!("{}\nhelp: {}", message, help_msg);
        self
    }
}

/// A result type for `es-fluent-derive-core`.
pub type EsFluentCoreResult<T> = Result<T, EsFluentCoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_error_display_includes_field_name_when_present() {
        let with_name = EsFluentCoreError::FieldError {
            message: "must not be empty".to_string(),
            field_name: Some("title".to_string()),
            span: None,
        };
        let without_name = EsFluentCoreError::FieldError {
            message: "must not be empty".to_string(),
            field_name: None,
            span: None,
        };

        assert_eq!(
            with_name.to_string(),
            "Field 'title' error: must not be empty"
        );
        assert_eq!(without_name.to_string(), "Field error: must not be empty");
    }

    #[test]
    fn core_error_span_and_message_mut_work_for_all_variants() {
        let span = proc_macro2::Span::call_site();
        let mut errors = [
            EsFluentCoreError::AttributeError {
                message: "a".to_string(),
                span: Some(span),
            },
            EsFluentCoreError::VariantError {
                message: "b".to_string(),
                variant_name: "MyVariant".to_string(),
                span: Some(span),
            },
            EsFluentCoreError::FieldError {
                message: "c".to_string(),
                field_name: Some("field".to_string()),
                span: Some(span),
            },
            EsFluentCoreError::TransformError {
                message: "d".to_string(),
                span: Some(span),
            },
        ];

        for (idx, err) in errors.iter_mut().enumerate() {
            assert!(err.span().is_some());
            *err.message_mut() = format!("updated-{idx}");
            assert!(err.to_string().contains(&format!("updated-{idx}")));
        }
    }

    #[test]
    fn error_ext_appends_note_and_help() {
        let err = EsFluentCoreError::AttributeError {
            message: "base".to_string(),
            span: None,
        }
        .with_note("extra context".to_string())
        .with_help("try this".to_string());

        let rendered = err.to_string();
        assert!(rendered.contains("base"));
        assert!(rendered.contains("note: extra context"));
        assert!(rendered.contains("help: try this"));
    }

    #[test]
    fn structured_attribute_error_includes_context_note_and_help() {
        let err = EsFluentCoreError::StructuredAttributeError(AttrError::new(
            AttrContext::MessageField,
            "Fluent argument name must not be empty",
            None,
        ))
        .with_note("field-level argument parsing".to_string())
        .with_help("use #[fluent(arg = \"name\")]".to_string());

        assert_eq!(
            err.to_string(),
            "Attribute error in message field: Fluent argument name must not be empty\nnote: field-level argument parsing\nhelp: use #[fluent(arg = \"name\")]"
        );
    }
}
