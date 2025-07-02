use proc_macro_error2::{abort, abort_call_site, emit_error};
use proc_macro2::Span;

#[derive(Debug, thiserror::Error)]
pub enum EsFluentCoreError {
    /// Error related to Fluent attribute parsing
    #[error("Attribute error: {message}")]
    AttributeError { message: String, span: Option<Span> },

    /// Error related to variant consistency
    #[error("Variant '{variant_name}' error: {message}")]
    VariantError {
        message: String,
        variant_name: String,
        span: Option<Span>,
    },

    /// Error related to field processing
    #[error("{}", field_error_fmt(.message, .field_name))]
    FieldError {
        message: String,
        field_name: Option<String>,
        span: Option<Span>,
    },

    /// Error with conversions or transformations
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
    pub fn span(&self) -> Option<Span> {
        match self {
            EsFluentCoreError::AttributeError { span, .. } => *span,
            EsFluentCoreError::VariantError { span, .. } => *span,
            EsFluentCoreError::FieldError { span, .. } => *span,
            EsFluentCoreError::TransformError { span, .. } => *span,
        }
    }

    pub fn message_mut(&mut self) -> &mut String {
        match self {
            EsFluentCoreError::AttributeError { message, .. } => message,
            EsFluentCoreError::VariantError { message, .. } => message,
            EsFluentCoreError::FieldError { message, .. } => message,
            EsFluentCoreError::TransformError { message, .. } => message,
        }
    }

    pub fn abort(self) -> ! {
        let msg = self.to_string();
        match self.span() {
            Some(span) => abort!(span, "{}", msg),
            None => abort_call_site!("{}", msg),
        }
    }

    pub fn emit(&self) {
        let msg = self.to_string();
        match self.span() {
            Some(span) => emit_error!(span, "{}", msg),
            None => emit_error!("{}", msg),
        }
    }
}

pub trait ErrorExt {
    fn with_note(self, note: String) -> Self;
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

pub type EsFluentCoreResult<T> = Result<T, crate::error::EsFluentCoreError>;
