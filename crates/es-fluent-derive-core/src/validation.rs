//! This module provides functions for validating `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::options::r#enum::EnumOpts;
use crate::options::namespace::NamespaceValue;
use crate::options::r#struct::StructOpts;
use crate::options::{
    EnumDataOptions as _, FluentField as _, StructDataOptions as _, VariantFields as _,
};
use es_fluent_toml::{I18nConfig, I18nConfigError};

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

        if field.is_skipped() && field.arg_name().is_some() {
            return Err(EsFluentCoreError::FieldError {
                message: "Cannot use #[fluent(arg_name = \"...\")] on a skipped field".to_string(),
                field_name: field.ident().as_ref().map(|i| i.to_string()),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }

        if let Some(arg_name) = field.arg_name()
            && arg_name.is_empty()
        {
            return Err(EsFluentCoreError::FieldError {
                message: "`#[fluent(arg_name = \"...\")]` cannot be empty".to_string(),
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

    // Ensure exposed argument names remain unique after arg_name overrides.
    let mut seen = std::collections::HashSet::new();
    for (index, field) in opts.indexed_fields() {
        let arg_name = field.fluent_arg_name(index);
        if !seen.insert(arg_name.clone()) {
            return Err(EsFluentCoreError::FieldError {
                message: format!(
                    "duplicate argument name '{}' after applying #[fluent(arg_name = \"...\")]",
                    arg_name
                ),
                field_name: Some(arg_name),
                span: field.ident().as_ref().map(|ident| ident.span()),
            });
        }
    }
    Ok(())
}

/// Validates enum-specific attributes.
pub fn validate_enum(opts: &EnumOpts) -> EsFluentCoreResult<()> {
    for variant in opts.variants() {
        let is_tuple = matches!(variant.style(), darling::ast::Style::Tuple);
        let variant_name = variant.ident().to_string();
        let variant_span = Some(variant.ident().span());
        let all_fields = variant.all_fields();
        let field_arg_name_overrides: Vec<_> = all_fields
            .iter()
            .filter_map(|field| field.arg_name().map(|name| (field, name)))
            .collect();

        if !field_arg_name_overrides.is_empty() {
            let mut explicit_seen = std::collections::HashSet::new();
            for (field, name) in &field_arg_name_overrides {
                if field.is_skipped() {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!(
                            "`#[fluent(arg_name = \"{}\")]` cannot be used on a skipped field",
                            name
                        ),
                        variant_name,
                        span: variant_span,
                    });
                }
                if name.is_empty() {
                    return Err(EsFluentCoreError::VariantError {
                        message: "`#[fluent(arg_name = \"...\")]` on fields cannot be empty"
                            .to_string(),
                        variant_name,
                        span: variant_span,
                    });
                }
                if !explicit_seen.insert(name.clone()) {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!("duplicate field arg_name '{}' in variant fields", name),
                        variant_name,
                        span: variant_span,
                    });
                }
            }

            let mut final_seen = std::collections::HashSet::new();

            for (tuple_index, field) in all_fields.iter().enumerate() {
                if field.is_skipped() {
                    continue;
                }

                let resolved_name = if let Some(name) = field.arg_name() {
                    name
                } else if is_tuple {
                    format!("f{}", tuple_index)
                } else {
                    field
                        .ident()
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| format!("f{}", tuple_index))
                };

                if !final_seen.insert(resolved_name.clone()) {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!(
                            "duplicate resolved argument name '{}' after applying #[fluent(arg_name = \"...\")]",
                            resolved_name
                        ),
                        variant_name,
                        span: variant_span,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Validates that a namespace is in the allowed list from `i18n.toml`.
///
/// - If `i18n.toml` doesn't exist or doesn't specify `namespaces`, validation passes.
/// - For `NamespaceValue::Literal`, validates against the configured namespaces.
/// - For path-derived namespaces (`File`, `FileRelative`, `Folder`, `FolderRelative`),
///   validation is deferred to the CLI since the source file path isn't reliably available at
///   macro expansion time.
///
/// Returns `Ok(())` if validation passes or should be deferred.
pub fn validate_namespace(
    namespace: &NamespaceValue,
    span: Option<proc_macro2::Span>,
) -> EsFluentCoreResult<()> {
    // Only validate literal namespaces at compile time
    let literal_value = match namespace {
        NamespaceValue::Literal(s) => s.as_ref(),
        // File-based namespaces need runtime/CLI validation
        NamespaceValue::File
        | NamespaceValue::FileRelative
        | NamespaceValue::Folder
        | NamespaceValue::FolderRelative => return Ok(()),
    };

    // Try to read the config; if it doesn't exist, skip validation
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

    // If namespaces aren't configured, allow any value
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
    allowed: &[String],
    span: Option<proc_macro2::Span>,
) -> EsFluentCoreResult<()> {
    if !allowed.contains(&namespace.to_string()) {
        return Err(EsFluentCoreError::AttributeError {
            message: format!(
                "namespace '{}' is not in the allowed list configured in i18n.toml",
                namespace
            ),
            span,
        }
        .with_help(format!("allowed namespaces are: {}", allowed.join(", "))));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validate_namespace_against_allowed_tests {
        use super::*;

        #[test]
        fn allowed_namespace_passes() {
            let allowed = vec![
                "ui".to_string(),
                "errors".to_string(),
                "messages".to_string(),
            ];
            validate_namespace_against_allowed("ui", &allowed, None)
                .expect("Should pass for allowed namespace");
            validate_namespace_against_allowed("errors", &allowed, None)
                .expect("Should pass for allowed namespace");
            validate_namespace_against_allowed("messages", &allowed, None)
                .expect("Should pass for allowed namespace");
        }

        #[test]
        fn disallowed_namespace_fails() {
            let allowed = vec!["ui".to_string(), "errors".to_string()];
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
            let allowed: Vec<String> = vec![];
            let err = validate_namespace_against_allowed("any", &allowed, None)
                .expect_err("Should fail when allowed list is empty");

            assert!(err.to_string().contains("not in the allowed list"));
        }

        #[test]
        fn case_sensitive_matching() {
            let allowed = vec!["UI".to_string()];
            let err = validate_namespace_against_allowed("ui", &allowed, None)
                .expect_err("Should fail for case mismatch");

            assert!(err.to_string().contains("ui"));
        }
    }

    mod validate_namespace_tests {
        use super::*;

        #[test]
        fn file_namespace_deferred() {
            let ns = NamespaceValue::File;
            validate_namespace(&ns, None).expect("File namespace should be deferred (always pass)");
        }

        #[test]
        fn file_relative_namespace_deferred() {
            let ns = NamespaceValue::FileRelative;
            validate_namespace(&ns, None)
                .expect("FileRelative namespace should be deferred (always pass)");
        }

        #[test]
        fn folder_namespace_deferred() {
            let ns = NamespaceValue::Folder;
            validate_namespace(&ns, None)
                .expect("Folder namespace should be deferred (always pass)");
        }

        #[test]
        fn folder_relative_namespace_deferred() {
            let ns = NamespaceValue::FolderRelative;
            validate_namespace(&ns, None)
                .expect("FolderRelative namespace should be deferred (always pass)");
        }
    }
}
