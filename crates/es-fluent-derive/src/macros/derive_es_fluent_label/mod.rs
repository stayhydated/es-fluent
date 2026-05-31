use darling::FromDeriveInput as _;
use es_fluent_derive_core::context::{ContainerContext, SpannedNamespaceRule};
use es_fluent_derive_core::error::AttrContext;
use es_fluent_derive_core::lowered::LabelModel;
use es_fluent_derive_core::semantic::{
    InventoryPolicy, MessageModel, RustSourceName, RustTypeName,
};
use es_fluent_derive_core::{options::label::LabelOpts, validation};
use es_fluent_shared::{meta::TypeKind, namespace::NamespaceRule};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::macros::ir::{MessageEntrySpec, inventory_variant_tokens_for_model};
use crate::macros::utils::{
    CodegenContext, InventoryModuleInput, NamespaceSource, SpannedNamespaceRuleRef,
};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let context = CodegenContext::resolve();
    expand_es_fluent_label_with_context(input, &context).into()
}

fn validate_namespace(
    namespace: Option<&NamespaceRule>,
    span: proc_macro2::Span,
) -> es_fluent_derive_core::error::EsFluentCoreResult<()> {
    if let Some(ns) = namespace
        && let Err(error) = validation::validate_namespace(ns, Some(span))
    {
        return Err(error);
    }

    Ok(())
}

#[cfg(test)]
fn expand_es_fluent_label(input: DeriveInput) -> proc_macro2::TokenStream {
    let context = CodegenContext::fallback();
    expand_es_fluent_label_with_context(input, &context)
}

fn expand_es_fluent_label_with_context(
    input: DeriveInput,
    context: &CodegenContext,
) -> proc_macro2::TokenStream {
    if let Err(err) = validation::validate_es_fluent_label_attribute_context(&input) {
        return crate::macros::utils::core_error_to_compile_error(err);
    }

    let opts = match LabelOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };

    let container_context = match ContainerContext::from_derive_input(&input) {
        Ok(context) => context,
        Err(err) => return err.write_errors(),
    };

    let model = match LabelModel::from_options(&opts) {
        Ok(model) => model,
        Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
    };

    let original_ident = model.ident();
    let generics = opts.generics();
    let ftl_key = if opts.attr_args().is_origin() {
        Some(model.message_id().clone())
    } else {
        None
    };

    let localize_label_impl = crate::macros::utils::generate_localize_label_impl(
        context,
        original_ident,
        generics,
        ftl_key.as_ref().map(|key| key.value()),
        container_context.fluent_domain(),
    );
    let (type_kind, semantic_type_kind) = match model.type_kind() {
        TypeKind::Struct => (
            {
                let es_fluent = context.facade_path().tokens();
                quote! { #es_fluent::meta::TypeKind::Struct }
            },
            TypeKind::Struct,
        ),
        TypeKind::Enum => (
            {
                let es_fluent = context.facade_path().tokens();
                quote! { #es_fluent::meta::TypeKind::Enum }
            },
            TypeKind::Enum,
        ),
    };

    // Generate inventory submission for types with origin=true
    // FTL metadata is purely structural and doesn't depend on generic type parameters
    let inventory_submit = if let Some(ftl_key) = &ftl_key {
        let label_namespace = opts.attr_args().namespace().map(|namespace| {
            SpannedNamespaceRuleRef::new(
                namespace,
                opts.attr_args()
                    .namespace_span()
                    .unwrap_or_else(|| original_ident.span()),
            )
        });
        let namespace = match crate::macros::utils::resolve_single_namespace_source([
            NamespaceSource::new(
                "#[fluent(namespace = ...)]",
                AttrContext::MessageContainer,
                container_context
                    .fluent_namespace()
                    .map(SpannedNamespaceRule::as_ref),
            ),
            NamespaceSource::new(
                "#[fluent_label(namespace = ...)]",
                AttrContext::LabelContainer,
                label_namespace,
            ),
        ]) {
            Ok(namespace) => namespace,
            Err(error) => return crate::macros::utils::core_error_to_compile_error(error),
        };
        if let Err(error) = validate_namespace(
            namespace.map(SpannedNamespaceRuleRef::rule),
            namespace
                .map(SpannedNamespaceRuleRef::span)
                .unwrap_or_else(|| original_ident.span()),
        ) {
            return crate::macros::utils::core_error_to_compile_error(error);
        }
        let label_entry = MessageEntrySpec::new(
            RustSourceName::from_ident(original_ident),
            ftl_key.clone(),
            Vec::new(),
        );
        let label_variant = inventory_variant_tokens_for_model(context, &label_entry.metadata);
        let label_model = MessageModel::new(
            RustTypeName::from_ident(original_ident),
            semantic_type_kind,
            None,
            namespace.map(|namespace| namespace.rule().clone()),
            Vec::new(),
            Some(label_entry.metadata),
            InventoryPolicy::Emit,
        );

        crate::macros::utils::generate_inventory_module(
            context,
            InventoryModuleInput {
                ident: original_ident,
                module_name_prefix: "label_inventory",
                type_kind,
                variants: vec![label_variant],
                namespace_expr: crate::macros::utils::namespace_rule_tokens(
                    context,
                    label_model.namespace(),
                ),
            },
        )
    } else {
        quote! {}
    };

    let tokens = quote! {
        #localize_label_impl
        #inventory_submit
    };

    tokens
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use insta::assert_snapshot;
    use syn::parse_quote;

    #[test]
    fn expand_es_fluent_label_generates_inventory_when_origin_is_enabled() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label]
            #[fluent(namespace = "ui")]
            struct LoginForm;
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert_snapshot!(
            "expand_es_fluent_label_generates_inventory_when_origin_is_enabled",
            tokens
        );
    }

    #[test]
    fn expand_es_fluent_label_skips_inventory_when_origin_is_disabled() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label(origin = false)]
            enum NoOrigin {
                A
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert_snapshot!(
            "expand_es_fluent_label_skips_inventory_when_origin_is_disabled",
            tokens
        );
    }

    #[test]
    fn expand_es_fluent_label_returns_compile_errors_for_parse_failures() {
        let label_opts_error: syn::DeriveInput = parse_quote! {
            #[fluent_label(origin = "nope")]
            struct InvalidLabelOpts;
        };
        let label_opts_tokens = crate::snapshot_support::pretty_file_tokens(
            super::expand_es_fluent_label(label_opts_error),
        );
        assert_snapshot!(
            "expand_es_fluent_label_returns_compile_errors_for_invalid_label_opts",
            label_opts_tokens
        );

        let struct_namespace_error: syn::DeriveInput = parse_quote! {
            #[fluent_label]
            #[fluent(namespace = 123)]
            struct InvalidStructNamespace;
        };
        let struct_tokens = crate::snapshot_support::pretty_file_tokens(
            super::expand_es_fluent_label(struct_namespace_error),
        );
        assert_snapshot!(
            "expand_es_fluent_label_returns_compile_errors_for_invalid_struct_namespace",
            struct_tokens
        );

        let enum_namespace_error: syn::DeriveInput = parse_quote! {
            #[fluent_label]
            #[fluent(namespace = 123)]
            enum InvalidEnumNamespace {
                A
            }
        };
        let enum_tokens = crate::snapshot_support::pretty_file_tokens(
            super::expand_es_fluent_label(enum_namespace_error),
        );
        assert_snapshot!(
            "expand_es_fluent_label_returns_compile_errors_for_invalid_enum_namespace",
            enum_tokens
        );
    }

    #[test]
    fn expand_es_fluent_label_rejects_parent_and_label_namespace_conflict() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent")]
            #[fluent_label(namespace = "child")]
            struct NamespacedLabel;
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("conflicting namespace declarations"));
        assert!(tokens.contains("#[fluent(namespace = ...)]"));
        assert!(tokens.contains("#[fluent_label(namespace = ...)]"));
    }

    #[test]
    fn expand_es_fluent_label_uses_struct_type_kind_for_structs() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label]
            struct LoginForm;
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert_snapshot!(
            "expand_es_fluent_label_uses_struct_type_kind_for_structs",
            tokens
        );
    }

    #[test]
    fn expand_es_fluent_label_uses_enum_type_kind_for_enums() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label]
            enum LoginState {
                Ready
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert_snapshot!(
            "expand_es_fluent_label_uses_enum_type_kind_for_enums",
            tokens
        );
    }
}
