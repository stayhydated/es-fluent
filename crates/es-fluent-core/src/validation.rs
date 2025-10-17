//! This module provides functions for validating `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::options::r#enum::EnumOpts;
use crate::options::r#struct::StructOpts;
use syn::{DataEnum, DataStruct};

/// Validates the `es-fluent` attributes on an enum.
pub fn validate_enum(_opts: &EnumOpts, _data: &DataEnum) -> EsFluentCoreResult<()> {
    Ok(())
}

/// Validates the `es-fluent` attributes on a struct.
pub fn validate_struct(opts: &StructOpts, _data: &DataStruct) -> EsFluentCoreResult<()> {
    validate_struct_defaults(opts)?;
    Ok(())
}

fn validate_struct_defaults(opts: &StructOpts) -> EsFluentCoreResult<()> {
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
