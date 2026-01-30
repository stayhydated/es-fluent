//! This module provides functions for validating `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::options::namespace::NamespaceValue;
use crate::options::r#struct::StructOpts;
use es_fluent_toml::I18nConfig;
use fluent_syntax::ast;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

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

/// Validates that a namespace is in the allowed list from `i18n.toml`.
///
/// - If `i18n.toml` doesn't exist or doesn't specify `namespaces`, validation passes.
/// - For `NamespaceValue::Literal`, validates against the configured namespaces.
/// - For `NamespaceValue::File` and `NamespaceValue::FileRelative`, validation is deferred
///   to the CLI since the source file path isn't reliably available at macro expansion time.
///
/// Returns `Ok(())` if validation passes or should be deferred.
pub fn validate_namespace(
    namespace: &NamespaceValue,
    span: Option<proc_macro2::Span>,
) -> EsFluentCoreResult<()> {
    // Only validate literal namespaces at compile time
    let literal_value = match namespace {
        NamespaceValue::Literal(s) => s,
        // File-based namespaces need runtime/CLI validation
        NamespaceValue::File | NamespaceValue::FileRelative => return Ok(()),
    };

    // Try to read the config; if it doesn't exist, skip validation
    let config = match I18nConfig::read_from_manifest_dir() {
        Ok(c) => c,
        Err(_) => return Ok(()), // No config file, no validation
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

/// Validates that all FTL keys exist in the FTL files when strict mode is enabled.
///
/// This function checks if the given keys exist in the fallback language's FTL files.
/// If strict mode is enabled in `i18n.toml` and any key is missing, it returns an error.
///
/// # Arguments
/// * `keys` - The FTL keys to validate
/// * `span` - Optional span for error reporting
///
/// # Returns
/// * `Ok(())` if strict mode is disabled or all keys exist
/// * `Err(EsFluentCoreError)` if strict mode is enabled and a key is missing
pub fn validate_keys_in_strict_mode(
    keys: &[String],
    span: Option<proc_macro2::Span>,
) -> EsFluentCoreResult<()> {
    // Try to read the config; if it doesn't exist or strict is false, skip validation
    let config = match I18nConfig::read_from_manifest_dir() {
        Ok(c) => c,
        Err(_) => return Ok(()), // No config file, no validation
    };

    // If strict mode is not enabled, skip validation
    if !config.strict {
        return Ok(());
    }

    // Get the fallback language and assets directory
    let fallback_lang = config.fallback_language_id();
    let assets_dir = match config.assets_dir_from_manifest() {
        Ok(dir) => dir,
        Err(_) => return Ok(()), // Can't find assets dir, skip validation
    };

    // Parse all FTL files in the fallback language directory
    let fallback_dir = assets_dir.join(fallback_lang);
    let ftl_keys = collect_all_ftl_keys(&fallback_dir);

    // Check if all keys exist
    for key in keys {
        if !ftl_keys.contains(key) {
            return Err(EsFluentCoreError::AttributeError {
                message: format!(
                    "FTL key '{}' is missing from the fallback language ('{}') FTL files",
                    key, fallback_lang
                ),
                span,
            }
            .with_help(format!(
                "Add the key to an FTL file in {}/ or set strict = false in i18n.toml",
                fallback_dir.display()
            )));
        }
    }

    Ok(())
}

/// Collect all message keys from FTL files in a directory (recursively).
fn collect_all_ftl_keys(dir: &Path) -> HashSet<String> {
    let mut keys = HashSet::new();

    if !dir.exists() {
        return keys;
    }

    collect_ftl_keys_recursive(dir, &mut keys);

    keys
}

/// Recursively collect message keys from FTL files.
fn collect_ftl_keys_recursive(dir: &Path, keys: &mut HashSet<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Recursively search subdirectories
            collect_ftl_keys_recursive(&path, keys);
        } else if path.extension().is_some_and(|ext| ext == "ftl") {
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            let resource = match fluent_syntax::parser::parse(content) {
                Ok(resource) => resource,
                Err((resource, _)) => resource, // Use partial result on parse errors
            };

            for entry in &resource.body {
                if let ast::Entry::Message(msg) = entry {
                    keys.insert(msg.id.name.clone());
                }
            }
        }
    }
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
    }
}
