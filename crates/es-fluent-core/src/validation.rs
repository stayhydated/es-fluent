//! This module provides functions for validating `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::options::r#struct::StructOpts;

/// Validates the `es-fluent` attributes on a struct.
/// Currently only checks that at most one field is marked `#[fluent(default)]`.
pub fn validate_struct(opts: &StructOpts) -> EsFluentCoreResult<()> {
    // Check for conflicting attributes on all fields
    for field in opts.all_indexed_fields().into_iter().map(|(_, f)| f) {
        if field.is_skipped() && field.is_default() {
            return Err(EsFluentCoreError::FieldError {
                message: "Cannot be both #[fluent(skip)] and #[fluent(default)]".to_string(),
                field_name: field.ident().as_ref().map(|i| i.to_string()),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }
    }

    let default_fields: Vec<_> = opts
        .indexed_fields()
        .into_iter()
        .filter(|(_, field)| field.is_default())
        .collect();

    if default_fields.len() > 1 {
        let (first_index, first_field) = &default_fields[0];
        let (second_index, second_field) = &default_fields[1];

        let first_field_name = first_field.fluent_arg_name(*first_index);
        let second_field_name = second_field.fluent_arg_name(*second_index);
        let second_span = second_field.ident().as_ref().map(|ident| ident.span());

        return Err(EsFluentCoreError::FieldError {
            message: "Struct cannot have multiple fields marked `#[fluent(default)]`.".to_string(),
            field_name: Some(second_field_name),
            span: second_span,
        }
        .with_note(format!(
            "First `#[fluent(default)]` field found was `{}`.",
            first_field_name
        )));
    }
    Ok(())
}
