//! This module provides functions for validating `es-fluent` attributes.

use crate::attribute::{AttributeLocation, AttributeName, validate_attribute_for_location};
use crate::error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::lowered::{
    GeneratedVariantsEnumModel, GeneratedVariantsStructModel, MessageEnumModel, MessageStructModel,
};
use crate::options::FluentField;
use crate::options::r#enum::EnumOpts;
use crate::options::r#struct::StructOpts;
use es_fluent_shared::{
    namer,
    namespace::{NamespaceRule, ResolvedNamespace},
};
use es_fluent_toml::{I18nConfig, I18nConfigError};
use heck::ToPascalCase as _;
use syn::{DeriveInput, spanned::Spanned as _};

#[derive(Clone, Copy, Debug)]
pub struct NamespaceSource<'a> {
    name: &'static str,
    context: AttrContext,
    namespace: Option<SpannedNamespaceRuleRef<'a>>,
}

impl<'a> NamespaceSource<'a> {
    pub fn new(
        name: &'static str,
        context: AttrContext,
        namespace: Option<SpannedNamespaceRuleRef<'a>>,
    ) -> Self {
        Self {
            name,
            context,
            namespace,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpannedNamespaceRuleRef<'a> {
    rule: &'a NamespaceRule,
    span: proc_macro2::Span,
}

impl<'a> SpannedNamespaceRuleRef<'a> {
    pub fn new(rule: &'a NamespaceRule, span: proc_macro2::Span) -> Self {
        Self { rule, span }
    }

    pub fn rule(self) -> &'a NamespaceRule {
        self.rule
    }

    pub fn span(self) -> proc_macro2::Span {
        self.span
    }
}

pub fn resolve_single_namespace_source<'a>(
    sources: impl IntoIterator<Item = NamespaceSource<'a>>,
) -> EsFluentCoreResult<Option<SpannedNamespaceRuleRef<'a>>> {
    let mut resolved: Option<NamespaceSource<'a>> = None;

    for source in sources {
        if source.namespace.is_none() {
            continue;
        }

        let Some(first) = resolved else {
            resolved = Some(source);
            continue;
        };

        return Err(EsFluentCoreError::StructuredAttributeError(
            AttrError::new(
                source.context,
                format!(
                    "conflicting namespace declarations: {} and {} both apply to the same generated output",
                    first.name, source.name
                ),
                source.namespace.map(SpannedNamespaceRuleRef::span),
            )
        )
        .with_help(format!(
            "keep exactly one namespace declaration for this output; remove either {} or {}",
            first.name, source.name
        )));
    }

    Ok(resolved.and_then(|source| source.namespace))
}

/// Validates raw `#[fluent(...)]` usage on an `EsFluent` derive input before
/// Darling parses the attributes.
pub fn validate_es_fluent_attribute_context(input: &DeriveInput) -> EsFluentCoreResult<()> {
    for attr in &input.attrs {
        validate_attribute_for_location(
            attr,
            AttributeName::Fluent,
            message_container_location(input),
            Some(&input.ident),
        )?;
    }

    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                for attr in &field.attrs {
                    validate_attribute_for_location(
                        attr,
                        AttributeName::Fluent,
                        AttributeLocation::MessageField,
                        field.ident.as_ref(),
                    )?;
                }
            }
        },
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                for attr in &variant.attrs {
                    validate_attribute_for_location(
                        attr,
                        AttributeName::Fluent,
                        AttributeLocation::EnumVariant,
                        Some(&variant.ident),
                    )?;
                }

                for field in &variant.fields {
                    for attr in &field.attrs {
                        validate_attribute_for_location(
                            attr,
                            AttributeName::Fluent,
                            AttributeLocation::MessageField,
                            field.ident.as_ref(),
                        )?;
                    }
                }
            }
        },
        syn::Data::Union(_) => {},
    }

    Ok(())
}

/// Validates raw attributes used by `EsFluentVariants` before Darling parses them.
pub fn validate_es_fluent_variants_attribute_context(
    input: &DeriveInput,
) -> EsFluentCoreResult<()> {
    for attr in &input.attrs {
        validate_attribute_for_location(
            attr,
            AttributeName::Fluent,
            variants_parent_location(input),
            Some(&input.ident),
        )?;
        validate_attribute_for_location(
            attr,
            AttributeName::FluentVariants,
            AttributeLocation::VariantsContainer,
            Some(&input.ident),
        )?;
        validate_attribute_for_location(
            attr,
            AttributeName::FluentLabel,
            AttributeLocation::LabelContainer,
            Some(&input.ident),
        )?;
    }

    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                for attr in &field.attrs {
                    validate_attribute_for_location(
                        attr,
                        AttributeName::FluentVariants,
                        AttributeLocation::VariantsField,
                        field.ident.as_ref(),
                    )?;
                }
            }
        },
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                for attr in &variant.attrs {
                    validate_attribute_for_location(
                        attr,
                        AttributeName::FluentVariants,
                        AttributeLocation::VariantsVariant,
                        Some(&variant.ident),
                    )?;
                }
            }
        },
        syn::Data::Union(_) => {},
    }

    Ok(())
}

/// Validates raw attributes used by `EsFluentLabel` before Darling parses them.
pub fn validate_es_fluent_label_attribute_context(input: &DeriveInput) -> EsFluentCoreResult<()> {
    for attr in &input.attrs {
        validate_attribute_for_location(
            attr,
            AttributeName::Fluent,
            label_parent_location(input),
            Some(&input.ident),
        )?;
        validate_attribute_for_location(
            attr,
            AttributeName::FluentLabel,
            AttributeLocation::LabelContainer,
            Some(&input.ident),
        )?;
    }

    Ok(())
}

/// Validates raw attributes used by `EsFluentChoice` before Darling parses them.
pub fn validate_es_fluent_choice_attribute_context(input: &DeriveInput) -> EsFluentCoreResult<()> {
    for attr in &input.attrs {
        validate_attribute_for_location(
            attr,
            AttributeName::FluentChoice,
            AttributeLocation::ChoiceContainer,
            Some(&input.ident),
        )?;
    }

    Ok(())
}

pub fn validate_struct(opts: &StructOpts) -> EsFluentCoreResult<()> {
    validate_message_struct_model(&MessageStructModel::from_options(opts)?)
}

pub(crate) fn validate_message_struct_model(
    model: &MessageStructModel<'_>,
) -> EsFluentCoreResult<()> {
    for field in model.all_indexed_fields().into_iter().map(|(_, f)| f) {
        field.argument_value_strategy(field_span(field))?;
    }

    // Ensure exposed argument names remain unique after arg overrides.
    let mut seen = std::collections::HashSet::new();
    for field in model.fields() {
        let arg = field.argument_model()?;
        if !seen.insert(arg.name().clone()) {
            return Err(EsFluentCoreError::FieldError {
                message: format!(
                    "duplicate argument name '{}' after applying #[fluent(arg = \"...\")]",
                    arg.name().as_str()
                ),
                field_name: Some(arg.name().as_str().to_string()),
                span: field.binding().map(|ident| ident.span()),
            });
        }
    }
    Ok(())
}

fn message_container_location(input: &DeriveInput) -> AttributeLocation {
    match &input.data {
        syn::Data::Struct(_) => AttributeLocation::MessageStructContainer,
        syn::Data::Enum(_) => AttributeLocation::MessageEnumContainer,
        syn::Data::Union(_) => AttributeLocation::MessageStructContainer,
    }
}

fn label_parent_location(input: &DeriveInput) -> AttributeLocation {
    match &input.data {
        syn::Data::Struct(_) => AttributeLocation::LabelStructParentContainer,
        syn::Data::Enum(_) => AttributeLocation::LabelEnumParentContainer,
        syn::Data::Union(_) => AttributeLocation::LabelStructParentContainer,
    }
}

fn variants_parent_location(input: &DeriveInput) -> AttributeLocation {
    match &input.data {
        syn::Data::Struct(_) => AttributeLocation::VariantsStructParentContainer,
        syn::Data::Enum(_) => AttributeLocation::VariantsEnumParentContainer,
        syn::Data::Union(_) => AttributeLocation::VariantsStructParentContainer,
    }
}

/// Validates enum-specific attributes.
pub fn validate_enum(opts: &EnumOpts) -> EsFluentCoreResult<()> {
    let model = MessageEnumModel::from_options(opts)?;
    validate_message_enum_model(&model)?;
    validate_message_enum_ids(&model)
}

pub(crate) fn validate_message_enum_model(model: &MessageEnumModel<'_>) -> EsFluentCoreResult<()> {
    for variant in model.variants() {
        let variant_name = variant.ident().to_string();
        let variant_span = Some(variant.ident().span());
        let all_fields = variant.all_fields();
        let mut field_arg_overrides = Vec::new();
        for field_model in &all_fields {
            let field = field_model.field();
            field.argument_value_strategy(field_span(field))?;

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

                let resolved_name = field_model.argument_model()?;

                if !final_seen.insert(resolved_name.name().clone()) {
                    return Err(EsFluentCoreError::VariantError {
                        message: format!(
                            "duplicate resolved argument name '{}' after applying #[fluent(arg = \"...\")]",
                            resolved_name.name().as_str()
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

fn field_span(field: &impl FluentField) -> proc_macro2::Span {
    field
        .ident()
        .map_or_else(|| field.ty().span(), syn::Ident::span)
}

fn validate_message_enum_ids(model: &MessageEnumModel<'_>) -> EsFluentCoreResult<()> {
    let mut seen = std::collections::HashMap::new();

    for variant in model
        .variants()
        .iter()
        .filter(|variant| !variant.is_skipped())
    {
        reject_duplicate_generated_value(
            &mut seen,
            variant.message_id().value().as_str().to_string(),
            variant.message_id().span(),
            AttrContext::EnumVariant,
            "message id",
        )?;
    }

    Ok(())
}

/// Validates generated variant names for a struct-backed `EsFluentVariants` model.
pub(crate) fn validate_generated_variants_struct_model(
    model: &GeneratedVariantsStructModel<'_>,
) -> EsFluentCoreResult<()> {
    let mut rust_idents = std::collections::HashMap::new();
    let mut key_fragments = std::collections::HashMap::new();

    for field in model.fields() {
        let source_name = namer::rust_ident_name(field.ident);
        reject_duplicate_generated_value(
            &mut rust_idents,
            source_name.to_pascal_case(),
            field.ident.span(),
            AttrContext::VariantsField,
            "Rust variant identifier",
        )?;
        reject_duplicate_generated_value(
            &mut key_fragments,
            source_name,
            field.ident.span(),
            AttrContext::VariantsField,
            "message id fragment",
        )?;
    }

    Ok(())
}

/// Validates generated variant names for an enum-backed `EsFluentVariants` model.
pub(crate) fn validate_generated_variants_enum_model(
    model: &GeneratedVariantsEnumModel<'_>,
) -> EsFluentCoreResult<()> {
    let mut rust_idents = std::collections::HashMap::new();
    let mut key_fragments = std::collections::HashMap::new();

    for variant in model.variants() {
        let source_name = namer::rust_ident_name(variant.ident);
        reject_duplicate_generated_value(
            &mut rust_idents,
            source_name.clone(),
            variant.ident.span(),
            AttrContext::VariantsContainer,
            "Rust variant identifier",
        )?;
        reject_duplicate_generated_value(
            &mut key_fragments,
            source_name,
            variant.ident.span(),
            AttrContext::VariantsContainer,
            "message id fragment",
        )?;
    }

    Ok(())
}

fn reject_duplicate_generated_value(
    seen: &mut std::collections::HashMap<String, proc_macro2::Span>,
    value: String,
    span: proc_macro2::Span,
    context: AttrContext,
    label: &str,
) -> EsFluentCoreResult<()> {
    if seen.contains_key(&value) {
        return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
            context,
            format!("generated {label} '{value}' collides with an earlier generated item"),
            Some(span),
        ))
        .with_note(format!(
            "first generated {label} '{value}' was seen earlier"
        ))
        .with_help("rename one source item or skip one generated item".to_string()));
    }

    seen.insert(value, span);
    Ok(())
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

    mod generated_collision_tests {
        use super::*;
        use crate::lowered::GeneratedVariantsStructModel;
        use crate::options::r#struct::StructVariantsOpts;
        use darling::FromDeriveInput as _;
        use syn::parse_quote;

        #[test]
        fn duplicate_enum_message_ids_are_rejected_before_emission() {
            let input: DeriveInput = parse_quote! {
                enum Status {
                    #[fluent(key = "same")]
                    One,
                    #[fluent(key = "same")]
                    Two,
                }
            };
            let opts = EnumOpts::from_derive_input(&input).expect("enum opts");

            let err = validate_enum(&opts).expect_err("duplicate message IDs should fail");

            assert!(
                err.to_string()
                    .contains("generated message id 'status-same' collides")
            );
        }

        #[test]
        fn struct_generated_variant_ident_collisions_are_rejected_before_emission() {
            let input: DeriveInput = parse_quote! {
                #[derive(EsFluentVariants)]
                struct Form {
                    foo_bar: String,
                    fooBar: String,
                }
            };
            let opts = StructVariantsOpts::from_derive_input(&input).expect("struct variants opts");
            let model = GeneratedVariantsStructModel::from_options(&opts).expect("lowered model");

            let err = validate_generated_variants_struct_model(&model)
                .expect_err("generated Rust variant collision should fail");

            assert!(
                err.to_string()
                    .contains("generated Rust variant identifier 'FooBar' collides")
            );
        }
    }

    mod namespace_source_tests {
        use super::*;

        #[test]
        fn resolve_single_namespace_source_rejects_multiple_sources() {
            let parent = NamespaceRule::Literal("parent".into());
            let child = NamespaceRule::Literal("child".into());

            let err = resolve_single_namespace_source([
                NamespaceSource::new(
                    "#[fluent(namespace = ...)]",
                    AttrContext::MessageContainer,
                    Some(SpannedNamespaceRuleRef::new(
                        &parent,
                        proc_macro2::Span::call_site(),
                    )),
                ),
                NamespaceSource::new(
                    "#[fluent_label(namespace = ...)]",
                    AttrContext::LabelContainer,
                    Some(SpannedNamespaceRuleRef::new(
                        &child,
                        proc_macro2::Span::call_site(),
                    )),
                ),
            ])
            .expect_err("multiple namespace sources should fail");

            assert!(
                err.to_string()
                    .contains("conflicting namespace declarations")
            );
            assert!(err.to_string().contains("#[fluent(namespace = ...)]"));
            assert!(err.to_string().contains("#[fluent_label(namespace = ...)]"));
        }

        #[test]
        fn resolve_single_namespace_source_returns_the_only_source() {
            let namespace = NamespaceRule::Literal("ui".into());
            let resolved = resolve_single_namespace_source([
                NamespaceSource::new(
                    "#[fluent(namespace = ...)]",
                    AttrContext::MessageContainer,
                    None,
                ),
                NamespaceSource::new(
                    "#[fluent_label(namespace = ...)]",
                    AttrContext::LabelContainer,
                    Some(SpannedNamespaceRuleRef::new(
                        &namespace,
                        proc_macro2::Span::call_site(),
                    )),
                ),
            ])
            .expect("single namespace source should pass")
            .expect("namespace");

            assert!(matches!(resolved.rule(), NamespaceRule::Literal(value) if value == "ui"));
        }
    }

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
