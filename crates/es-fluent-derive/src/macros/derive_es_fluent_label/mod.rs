use darling::FromDeriveInput as _;
use es_fluent_derive_core::{options::this::LabelOpts, validation};
use es_fluent_shared::{namer, namespace::NamespaceRule};
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::utils::InventoryModuleInput;

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_es_fluent_label(input).into()
}

fn validate_namespace(namespace: Option<&NamespaceRule>, span: proc_macro2::Span) {
    if let Some(ns) = namespace
        && let Err(err) = validation::validate_namespace(ns, Some(span))
    {
        err.abort();
    }
}

fn expand_es_fluent_label(input: DeriveInput) -> proc_macro2::TokenStream {
    let opts = match LabelOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };

    if matches!(&input.data, Data::Union(_)) {
        proc_macro_error2::abort!(
            input.ident.span(),
            "EsFluentLabel can only be derived for structs and enums"
        );
    }

    let fluent_namespace = match crate::macros::utils::inherited_fluent_namespace(&input) {
        Ok(namespace) => namespace,
        Err(err) => return err.write_errors(),
    };
    let fluent_domain = match crate::macros::utils::inherited_fluent_domain(&input) {
        Ok(domain) => domain,
        Err(err) => return err.write_errors(),
    };

    let original_ident = opts.ident();
    let generics = opts.generics();
    let ftl_key = if opts.attr_args().is_origin() {
        Some(namer::FluentKey::new_label(original_ident).to_string())
    } else {
        None
    };

    let localize_label_impl = crate::macros::utils::generate_localize_label_impl(
        original_ident,
        generics,
        ftl_key.as_deref(),
        fluent_domain.as_deref(),
    );
    let type_kind = match &input.data {
        Data::Struct(_) => quote! { ::es_fluent::meta::TypeKind::Struct },
        Data::Enum(_) => quote! { ::es_fluent::meta::TypeKind::Enum },
        Data::Union(_) => unreachable!("EsFluentLabel does not support unions"),
    };

    // Generate inventory submission for types with origin=true
    // FTL metadata is purely structural and doesn't depend on generic type parameters
    let inventory_submit = if let Some(ftl_key_str) = &ftl_key {
        let type_name = namer::rust_ident_name(original_ident);
        let namespace = crate::macros::utils::preferred_namespace([
            fluent_namespace.as_ref(),
            opts.attr_args().namespace(),
        ]);
        validate_namespace(namespace, original_ident.span());
        let namespace_expr = crate::macros::utils::namespace_rule_tokens(namespace);
        let label_variant = quote! {
            ::es_fluent::registry::FtlVariant {
                name: #type_name,
                ftl_key: #ftl_key_str,
                args: &[],
                module_path: module_path!(),
                line: line!(),
            }
        };

        crate::macros::utils::generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "label_inventory",
            type_kind,
            variants: vec![label_variant],
            namespace_expr,
        })
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
        let this_opts_error: syn::DeriveInput = parse_quote! {
            #[fluent_label(origin = "nope")]
            struct InvalidLabelOpts;
        };
        let this_opts_tokens = crate::snapshot_support::pretty_file_tokens(
            super::expand_es_fluent_label(this_opts_error),
        );
        assert_snapshot!(
            "expand_es_fluent_label_returns_compile_errors_for_invalid_label_opts",
            this_opts_tokens
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
    fn expand_es_fluent_label_prefers_parent_fluent_namespace_over_label_namespace() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent")]
            #[fluent_label(namespace = "child")]
            struct NamespacedLabel;
        };

        let tokens =
            crate::snapshot_support::pretty_file_tokens(super::expand_es_fluent_label(input));
        assert_snapshot!(
            "expand_es_fluent_label_prefers_parent_fluent_namespace_over_label_namespace",
            tokens
        );
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
