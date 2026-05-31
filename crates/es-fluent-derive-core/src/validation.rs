//! This module provides functions for validating `es-fluent` attributes.

use crate::attribute::{AttributeLocation, invalid_fluent_meta_item_for_location};
use crate::error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::lowered::{MessageEnumModel, MessageStructModel};
use crate::options::FluentField;
use crate::options::r#enum::EnumOpts;
use crate::options::r#struct::StructOpts;
use crate::semantic::{ArgName, SpannedValue};
use es_fluent_shared::namespace::{NamespaceRule, ResolvedNamespace};
use es_fluent_toml::{I18nConfig, I18nConfigError};
use syn::{DeriveInput, Meta, Token, punctuated::Punctuated, spanned::Spanned as _};

/// Validates raw `#[fluent(...)]` usage on an `EsFluent` derive input before
/// Darling parses the attributes.
pub fn validate_es_fluent_attribute_context(input: &DeriveInput) -> EsFluentCoreResult<()> {
    let syn::Data::Enum(data) = &input.data else {
        return Ok(());
    };

    for variant in &data.variants {
        for attr in variant
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("fluent"))
        {
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            let items = match list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            {
                Ok(items) => items,
                Err(_) => continue,
            };

            for item in items {
                if let Some(attr) =
                    invalid_fluent_meta_item_for_location(&item, AttributeLocation::EnumVariant)
                {
                    return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
                        AttrContext::EnumVariant,
                        format!(
                            "`{}` is a field-only attribute and cannot be used on enum variant `{}`",
                            attr.syntax(),
                            variant.ident
                        ),
                        Some(item.span()),
                    ))
                    .with_help(
                        format!(
                            "move the attribute to a field inside the variant, for example `{}(#[fluent(arg = \"name\")] T)`",
                            variant.ident
                        ),
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Validates the `es-fluent` attributes on a struct.
/// Currently only checks that at most one field is marked `#[fluent(default)]`.
pub fn validate_struct(opts: &StructOpts) -> EsFluentCoreResult<()> {
    validate_message_struct_model(&MessageStructModel::from_options(opts)?)
}

pub fn validate_message_struct_model(model: &MessageStructModel<'_>) -> EsFluentCoreResult<()> {
    // Check for conflicting attributes on all fields
    for field in model.all_indexed_fields().into_iter().map(|(_, f)| f) {
        if field.is_skipped() && field.is_default() {
            return Err(EsFluentCoreError::FieldError {
                message: "Cannot be both #[fluent(skip)] and #[fluent(default)]".to_string(),
                field_name: field.ident().as_ref().map(|i| i.to_string()),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }

        let explicit_arg = field.arg_name(AttrContext::MessageField)?;

        if field.is_skipped() && explicit_arg.is_some() {
            return Err(EsFluentCoreError::FieldError {
                message: "Cannot use #[fluent(arg = \"...\")] on a skipped field".to_string(),
                field_name: field.ident().as_ref().map(|i| i.to_string()),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }

        if field.is_choice() && field.value().is_some() {
            return Err(EsFluentCoreError::FieldError {
                message:
                    "Cannot combine #[fluent(choice)] and #[fluent(value = ...)] on the same field"
                        .to_string(),
                field_name: field.ident().as_ref().map(|i| i.to_string()),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }
    }

    let default_fields: Vec<_> = model
        .indexed_fields()
        .into_iter()
        .filter(|(_, field)| field.is_default())
        .collect();

    if default_fields.len() > 1 {
        let (first_index, first_field) = &default_fields[0];
        let (second_index, second_field) = &default_fields[1];

        let first_field_name =
            first_field.fluent_arg_name(*first_index, AttrContext::MessageField)?;
        let second_field_name =
            second_field.fluent_arg_name(*second_index, AttrContext::MessageField)?;
        let second_span = second_field.ident().as_ref().map(|ident| ident.span());

        return Err(EsFluentCoreError::FieldError {
            message: "Struct cannot have multiple fields marked `#[fluent(default)]`.".to_string(),
            field_name: Some(second_field_name.value().as_str().to_string()),
            span: second_span,
        }
        .with_note(format!(
            "First `#[fluent(default)]` field found was `{}`.",
            first_field_name.value().as_str()
        )));
    }

    // Ensure exposed argument names remain unique after arg overrides.
    let mut seen = std::collections::HashSet::new();
    for (index, field) in model.indexed_fields() {
        let arg = field.fluent_arg_name(index, AttrContext::MessageField)?;
        if !seen.insert(arg.value().clone()) {
            return Err(EsFluentCoreError::FieldError {
                message: format!(
                    "duplicate argument name '{}' after applying #[fluent(arg = \"...\")]",
                    arg.value().as_str()
                ),
                field_name: Some(arg.value().as_str().to_string()),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }
    }
    Ok(())
}

/// Validates enum-specific attributes.
pub fn validate_enum(opts: &EnumOpts) -> EsFluentCoreResult<()> {
    validate_message_enum_model(&MessageEnumModel::from_options(opts)?)
}

pub fn validate_message_enum_model(model: &MessageEnumModel<'_>) -> EsFluentCoreResult<()> {
    for variant in model.variants() {
        let variant_name = variant.ident().to_string();
        let variant_span = Some(variant.ident().span());
        let all_fields = variant.all_fields();
        let mut field_arg_overrides = Vec::new();
        for field_model in &all_fields {
            let field = field_model.field();
            if field.is_choice() && field.value().is_some() {
                return Err(EsFluentCoreError::VariantError {
                    message: "Cannot combine #[fluent(choice)] and #[fluent(value = ...)] on the same field".to_string(),
                    variant_name: variant_name.clone(),
                    span: variant_span,
                });
            }

            if let Some(arg) = field.arg_name(AttrContext::MessageField)? {
                field_arg_overrides.push((*field_model, arg));
            }
        }

        if !field_arg_overrides.is_empty() {
            let mut explicit_seen = std::collections::HashSet::new();
            for (field_model, name) in &field_arg_overrides {
                let field = field_model.field();
                if field.is_skipped() {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!(
                            "`#[fluent(arg = \"{}\")]` cannot be used on a skipped field",
                            name.value().as_str()
                        ),
                        variant_name: variant_name.clone(),
                        span: variant_span,
                    });
                }
                if !explicit_seen.insert(name.value().clone()) {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!(
                            "duplicate field arg '{}' in variant fields",
                            name.value().as_str()
                        ),
                        variant_name: variant_name.clone(),
                        span: variant_span,
                    });
                }
            }

            let mut final_seen = std::collections::HashSet::new();

            for field_model in &all_fields {
                let field = field_model.field();
                if field.is_skipped() {
                    continue;
                }

                let resolved_name = resolved_enum_field_arg_name(
                    field_model.field(),
                    field_model.declaration_index(),
                )?;

                if !final_seen.insert(resolved_name.value().clone()) {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!(
                            "duplicate resolved argument name '{}' after applying #[fluent(arg = \"...\")]",
                            resolved_name.value().as_str()
                        ),
                        variant_name: variant_name.clone(),
                        span: variant_span,
                    });
                }
            }
        }
    }

    Ok(())
}

fn resolved_enum_field_arg_name(
    field: &impl FluentField,
    index: usize,
) -> EsFluentCoreResult<SpannedValue<ArgName>> {
    field.fluent_arg_name(index, AttrContext::MessageField)
}

/// Validates that a namespace is in the allowed list from `i18n.toml`.
///
/// - If `i18n.toml` doesn't exist or doesn't specify `namespaces`, validation passes.
/// - For `NamespaceRule::Literal`, validates namespace path safety and then validates against
///   configured namespaces when an allowlist exists.
/// - For path-derived namespaces (`File`, `FileRelative`, `Folder`, `FolderRelative`),
///   validation is deferred to the CLI since the source file path isn't reliably available at
///   macro expansion time.
///
/// Returns `Ok(())` if validation passes or should be deferred.
pub fn validate_namespace(
    namespace: &NamespaceRule,
    span: Option<proc_macro2::Span>,
) -> EsFluentCoreResult<()> {
    // Only validate literal namespaces at compile time
    let literal_value = match namespace {
        NamespaceRule::Literal(s) => s.as_ref(),
        // File-based namespaces need runtime/CLI validation
        NamespaceRule::File
        | NamespaceRule::FileRelative
        | NamespaceRule::Folder
        | NamespaceRule::FolderRelative => return Ok(()),
    };

    if let Err(error) = ResolvedNamespace::new(literal_value) {
        return Err(EsFluentCoreError::AttributeError {
            message: format!("invalid namespace '{}': {}", literal_value, error),
            span,
        }
        .with_help(
            "use a relative namespace path such as \"ui\" or \"user/profile\"".to_string(),
        ));
    }

    // Try to read the config; if it doesn't exist, skip allowlist validation
    let config = match I18nConfig::read_from_manifest_dir() {
        Ok(c) => c,
        Err(I18nConfigError::NotFound) => return Ok(()),
        Err(error) => {
            return Err(EsFluentCoreError::AttributeError {
                message: format!(
                    "failed to read i18n.toml while validating namespace '{}': {}",
                    literal_value, error
                ),
                span,
            }
            .with_help(
                "fix the i18n.toml configuration or remove it to skip namespace allowlist validation"
                    .to_string(),
            ));
        },
    };

    // If namespaces aren't configured, allow any safe namespace value
    let allowed = match &config.namespaces {
        Some(ns) => ns,
        None => return Ok(()),
    };

    validate_namespace_against_allowed(literal_value, allowed, span)
}

/// Core validation logic for checking a namespace against an allowed list.
/// Extracted for testability.
pub fn validate_namespace_against_allowed(
    namespace: &str,
    allowed: &[ResolvedNamespace],
    span: Option<proc_macro2::Span>,
) -> EsFluentCoreResult<()> {
    if !allowed
        .iter()
        .any(|allowed_namespace| allowed_namespace.as_str() == namespace)
    {
        let allowed_list = allowed
            .iter()
            .map(ResolvedNamespace::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(EsFluentCoreError::AttributeError {
            message: format!(
                "namespace '{}' is not in the allowed list configured in i18n.toml",
                namespace
            ),
            span,
        }
        .with_help(format!("allowed namespaces are: {}", allowed_list)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validate_namespace_against_allowed_tests {
        use super::*;

        fn allowed(namespaces: &[&str]) -> Vec<ResolvedNamespace> {
            namespaces
                .iter()
                .copied()
                .map(ResolvedNamespace::new)
                .collect::<Result<_, _>>()
                .expect("test namespaces")
        }

        #[test]
        fn allowed_namespace_passes() {
            let allowed = allowed(&["ui", "errors", "messages"]);
            validate_namespace_against_allowed("ui", &allowed, None)
                .expect("Should pass for allowed namespace");
            validate_namespace_against_allowed("errors", &allowed, None)
                .expect("Should pass for allowed namespace");
            validate_namespace_against_allowed("messages", &allowed, None)
                .expect("Should pass for allowed namespace");
        }

        #[test]
        fn disallowed_namespace_fails() {
            let allowed = allowed(&["ui", "errors"]);
            let err = validate_namespace_against_allowed("unknown", &allowed, None)
                .expect_err("Should fail for disallowed namespace");

            let err_msg = err.to_string();
            assert!(err_msg.contains("unknown"));
            assert!(err_msg.contains("not in the allowed list"));
            assert!(err_msg.contains("ui"));
            assert!(err_msg.contains("errors"));
        }

        #[test]
        fn empty_allowed_list_rejects_all() {
            let allowed: Vec<ResolvedNamespace> = vec![];
            let err = validate_namespace_against_allowed("any", &allowed, None)
                .expect_err("Should fail when allowed list is empty");

            assert!(err.to_string().contains("not in the allowed list"));
        }

        #[test]
        fn case_sensitive_matching() {
            let allowed = allowed(&["UI"]);
            let err = validate_namespace_against_allowed("ui", &allowed, None)
                .expect_err("Should fail for case mismatch");

            assert!(err.to_string().contains("ui"));
        }
    }

    mod validate_namespace_tests {
        use super::*;

        #[test]
        fn file_namespace_deferred() {
            let ns = NamespaceRule::File;
            validate_namespace(&ns, None).expect("File namespace should be deferred (always pass)");
        }

        #[test]
        fn file_relative_namespace_deferred() {
            let ns = NamespaceRule::FileRelative;
            validate_namespace(&ns, None)
                .expect("FileRelative namespace should be deferred (always pass)");
        }

        #[test]
        fn folder_namespace_deferred() {
            let ns = NamespaceRule::Folder;
            validate_namespace(&ns, None)
                .expect("Folder namespace should be deferred (always pass)");
        }

        #[test]
        fn folder_relative_namespace_deferred() {
            let ns = NamespaceRule::FolderRelative;
            validate_namespace(&ns, None)
                .expect("FolderRelative namespace should be deferred (always pass)");
        }

        #[test]
        fn literal_namespace_rejects_unsafe_path() {
            let ns = NamespaceRule::Literal("../outside".into());
            let err = validate_namespace(&ns, None)
                .expect_err("Literal namespaces should reject unsafe paths");

            assert!(err.to_string().contains("invalid namespace"));
        }
    }
}
