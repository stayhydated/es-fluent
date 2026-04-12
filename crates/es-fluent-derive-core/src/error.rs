//! This module provides the error types for `es-fluent-derive-core`.

use proc_macro_error2::{abort, abort_call_site, emit_error};
use proc_macro2::Span;

pub use es_fluent_shared::error::{EsFluentError, EsFluentResult};

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
        let mut errors = vec![
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
    fn emit_and_abort_paths_cover_spans_and_call_site() {
        let span = proc_macro2::Span::call_site();
        let emit_with_span = std::panic::catch_unwind(|| {
            EsFluentCoreError::AttributeError {
                message: "emit with span".to_string(),
                span: Some(span),
            }
            .emit();
        });
        assert!(emit_with_span.is_err());

        let emit_without_span = std::panic::catch_unwind(|| {
            EsFluentCoreError::AttributeError {
                message: "emit without span".to_string(),
                span: None,
            }
            .emit();
        });
        assert!(emit_without_span.is_err());

        let abort_with_span = std::panic::catch_unwind(|| {
            EsFluentCoreError::AttributeError {
                message: "abort with span".to_string(),
                span: Some(span),
            }
            .abort();
        });
        assert!(abort_with_span.is_err());

        let abort_without_span = std::panic::catch_unwind(|| {
            EsFluentCoreError::AttributeError {
                message: "abort without span".to_string(),
                span: None,
            }
            .abort();
        });
        assert!(abort_without_span.is_err());
    }
}
