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
    let fields = opts.fields();
    let default_fields: Vec<_> = fields.iter().filter(|f| f.is_default()).collect();

    if default_fields.len() > 1
        && let Some(first_field_ident) = default_fields[0].ident().as_ref()
        && let Some(second_field_ident) = default_fields[1].ident().as_ref()
    {
        return Err(EsFluentCoreError::FieldError {
            message: "Struct cannot have multiple fields marked `#[fluent(default)]`.".to_string(),
            field_name: Some(second_field_ident.to_string()),
            span: Some(second_field_ident.span()),
        }
        .with_note(format!(
            "First `#[fluent(default)]` field found was `{}`.",
            first_field_ident
        )));
    }
    Ok(())
}
