//! This module provides functions for validating `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::options::r#enum::{EnumKvOpts, EnumOpts};
use crate::options::r#struct::{StructKvOpts, StructOpts};
use syn::{DataEnum, DataStruct};

/// Validates the `es-fluent` attributes on an enum.
pub fn validate_enum(opts: &EnumOpts, data: &DataEnum) -> EsFluentCoreResult<()> {
    validate_empty_enum(opts, data)?;
    Ok(())
}

/// Validates the `es-fluent` attributes on a struct.
pub fn validate_struct(opts: &StructOpts, data: &DataStruct) -> EsFluentCoreResult<()> {
    validate_empty_struct(opts, data)?;
    validate_struct_defaults(opts)?;
    Ok(())
}

/// Validates the `es-fluent_kv` attributes on a struct.
pub fn validate_struct_kv(opts: &StructKvOpts, data: &DataStruct) -> EsFluentCoreResult<()> {
    validate_empty_struct_kv(opts, data)?;
    Ok(())
}

/// Validates the `es-fluent_kv` attributes on an enum.
pub fn validate_enum_kv(opts: &EnumKvOpts, data: &DataEnum) -> EsFluentCoreResult<()> {
    validate_empty_enum_kv(opts, data)?;
    validate_enum_kv_variants(opts)?;
    Ok(())
}

fn validate_empty_enum(opts: &EnumOpts, data: &DataEnum) -> EsFluentCoreResult<()> {
    if data.variants.is_empty() && !opts.attr_args().is_this() {
        return Err(EsFluentCoreError::AttributeError {
            message: "Empty enum must have `#[fluent(this)]` attribute.".to_string(),
            span: Some(opts.ident().span()),
        }
        .with_help(
            "For empty enums, the only reflectable value is the type name itself. \
             Add `#[fluent(this)]` to generate a `this_ftl()` method."
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_empty_struct(opts: &StructOpts, data: &DataStruct) -> EsFluentCoreResult<()> {
    let is_empty = match &data.fields {
        syn::Fields::Named(fields) => fields.named.is_empty(),
        syn::Fields::Unnamed(fields) => fields.unnamed.is_empty(),
        syn::Fields::Unit => true,
    };

    if is_empty && !opts.attr_args().is_this() {
        return Err(EsFluentCoreError::AttributeError {
            message: "Empty struct must have `#[fluent(this)]` attribute.".to_string(),
            span: Some(opts.ident().span()),
        }
        .with_help(
            "For empty structs, the only reflectable value is the type name itself. \
             Add `#[fluent(this)]` to generate a `this_ftl()` method."
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_empty_struct_kv(opts: &StructKvOpts, data: &DataStruct) -> EsFluentCoreResult<()> {
    let is_empty = match &data.fields {
        syn::Fields::Named(fields) => fields.named.is_empty(),
        syn::Fields::Unnamed(fields) => fields.unnamed.is_empty(),
        syn::Fields::Unit => true,
    };

    let has_this_or_keys_this = opts.attr_args().is_this() || opts.attr_args().is_keys_this();

    if is_empty && !has_this_or_keys_this {
        return Err(EsFluentCoreError::AttributeError {
            message: "Empty struct must have `#[fluent_kv(this)]` or `#[fluent_kv(keys_this)]` attribute.".to_string(),
            span: Some(opts.ident().span()),
        }
        .with_help(
            "For empty structs, the only reflectable value is the type name itself. \
             Add `#[fluent_kv(this)]` to generate a `this_ftl()` method on the original type, \
             or `#[fluent_kv(keys_this)]` to generate it on the generated KV enums."
                .to_string(),
        ));
    }
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

fn validate_empty_enum_kv(opts: &EnumKvOpts, data: &DataEnum) -> EsFluentCoreResult<()> {
    let has_this_or_keys_this = opts.attr_args().is_this() || opts.attr_args().is_keys_this();
    if data.variants.is_empty() && !has_this_or_keys_this {
        return Err(EsFluentCoreError::AttributeError {
            message:
                "Empty enum must have `#[fluent_kv(this)]` or `#[fluent_kv(keys_this)]` attribute."
                    .to_string(),
            span: Some(opts.ident().span()),
        }
        .with_help(
            "For empty enums, the only reflectable value is the type name itself. \
             Add `#[fluent_kv(this)]` to generate a `this_ftl()` method on the generated KV enums, \
             or `#[fluent_kv(keys_this)]` to generate it on the original type."
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_enum_kv_variants(opts: &EnumKvOpts) -> EsFluentCoreResult<()> {
    for variant in opts.variants() {
        if !variant.is_single_tuple() {
            return Err(EsFluentCoreError::AttributeError {
                message: format!(
                    "EsFluentKv on enums only supports single-element tuple variants; \
                     variant `{}` is not a single-element tuple.",
                    variant.ident()
                ),
                span: Some(variant.ident().span()),
            }
            .with_help(
                "Each variant must have exactly one tuple field, e.g., `Variant(InnerType)`."
                    .to_string(),
            ));
        }
    }
    Ok(())
}
