use es_fluent_derive_core::expansion::{EsFluentLabelExpansion, ExpansionError};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::macros::utils::{CodegenContext, InventoryOutput};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let context = CodegenContext::resolve();
    expand_es_fluent_label_with_context(input, &context).into()
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
    let expansion = match EsFluentLabelExpansion::from_derive_input(&input) {
        Ok(expansion) => expansion,
        Err(ExpansionError::Core(error)) => {
            return crate::macros::utils::core_error_to_compile_error(error);
        },
        Err(ExpansionError::Darling(error)) => return error.write_errors(),
        Err(ExpansionError::Syn(error)) => return error.to_compile_error(),
    };

    let localize_label_impl = crate::macros::utils::generate_localize_label_impl(
        context,
        expansion.ident(),
        expansion.generics(),
        expansion.ftl_key(),
        expansion.domain(),
    );
    let label_origin_marker_impl = crate::macros::utils::generate_label_origin_marker_impl(
        context,
        expansion.ident(),
        expansion.generics(),
        expansion.ftl_key().is_some(),
    );

    let inventory_output = if let Some(label_model) = expansion.label_inventory() {
        if let Some(label) = label_model.label() {
            crate::macros::utils::label_inventory_output(
                expansion.ident(),
                label_model.type_kind().clone(),
                label_model.namespace().cloned(),
                label.clone(),
            )
        } else {
            InventoryOutput::None
        }
    } else {
        InventoryOutput::None
    };
    let inventory_submit = crate::macros::utils::emit_inventory_output(context, inventory_output);

    let tokens = quote! {
        #localize_label_impl
        #label_origin_marker_impl
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
            #[fluent_label(origin)]
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
    fn expand_es_fluent_label_rejects_missing_origin() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label]
            struct MissingOrigin;
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert!(tokens.contains("compile_error"));
        assert!(tokens.contains("requires `#[fluent_label(origin)]`"));
    }

    #[test]
    fn expand_es_fluent_label_rejects_explicit_boolean_origin() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_label(origin = false)]
            enum NoOrigin {
                A
            }
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert_snapshot!(
            "expand_es_fluent_label_rejects_explicit_boolean_origin",
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
            #[fluent_label(origin)]
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
            #[fluent_label(origin)]
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
            #[fluent_label(origin, namespace = "child")]
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
            #[fluent_label(origin)]
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
            #[fluent_label(origin)]
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
