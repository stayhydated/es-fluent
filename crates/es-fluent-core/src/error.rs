//! This module provides the error types for `es-fluent-core`.

use proc_macro_error2::{abort, abort_call_site, emit_error};
use proc_macro2::Span;

/// An error that can occur when parsing `es-fluent` attributes.
#[derive(Debug, thiserror::Error, Clone)]
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

/// A result type for `es-fluent-core`.
pub type EsFluentCoreResult<T> = Result<T, EsFluentCoreError>;
