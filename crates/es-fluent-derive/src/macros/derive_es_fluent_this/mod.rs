use darling::FromDeriveInput as _;
use es_fluent_derive_core::options::this::ThisOpts;
use es_fluent_shared::namer;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::utils::{
    InventoryModuleInput, generate_inventory_module, generate_this_ftl_impl,
    inherited_fluent_namespace, namespace_rule_tokens, preferred_namespace,
};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_es_fluent_this(input).into()
}

fn expand_es_fluent_this(input: DeriveInput) -> proc_macro2::TokenStream {
    let opts = match ThisOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };

    let fluent_namespace = match inherited_fluent_namespace(&input) {
        Ok(namespace) => namespace,
        Err(err) => return err.write_errors(),
    };

    let original_ident = opts.ident();
    let generics = opts.generics();
    let ftl_key = if opts.attr_args().is_origin() {
        Some(namer::FluentKey::new_this(original_ident).to_string())
    } else {
        None
    };

    let this_ftl_impl = generate_this_ftl_impl(original_ident, generics, ftl_key.as_deref());
    let type_kind = match &input.data {
        Data::Struct(_) => quote! { ::es_fluent::meta::TypeKind::Struct },
        Data::Enum(_) => quote! { ::es_fluent::meta::TypeKind::Enum },
        Data::Union(_) => unreachable!("EsFluentThis does not support unions"),
    };

    // Generate inventory submission for types with origin=true
    // FTL metadata is purely structural and doesn't depend on generic type parameters
    let inventory_submit = if let Some(ftl_key_str) = &ftl_key {
        let type_name = original_ident.to_string();
        let namespace_expr = namespace_rule_tokens(preferred_namespace([
            fluent_namespace.as_ref(),
            opts.attr_args().namespace(),
        ]));
        let this_variant = quote! {
            ::es_fluent::registry::FtlVariant {
                name: #type_name,
                ftl_key: #ftl_key_str,
                args: &[],
                module_path: module_path!(),
                line: line!(),
            }
        };

        generate_inventory_module(InventoryModuleInput {
            ident: original_ident,
            module_name_prefix: "this_inventory",
            type_kind,
            variants: vec![this_variant],
            namespace_expr,
        })
    } else {
        quote! {}
    };

    let tokens = quote! {
        #this_ftl_impl
        #inventory_submit
    };

    tokens
}

#[cfg(test)]
mod tests {
    use super::expand_es_fluent_this;
    use syn::parse_quote;

    #[test]
    fn expand_es_fluent_this_generates_inventory_when_origin_is_enabled() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_this]
            #[fluent(namespace = "ui")]
            struct LoginForm;
        };

        let tokens = expand_es_fluent_this(input).to_string();
        assert!(tokens.contains("__es_fluent_this_inventory_login_form"));
        assert!(tokens.contains("login_form_this"));
    }

    #[test]
    fn expand_es_fluent_this_skips_inventory_when_origin_is_disabled() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_this(origin = false)]
            enum NoOrigin {
                A
            }
        };

        let tokens = expand_es_fluent_this(input).to_string();
        assert!(!tokens.contains("__es_fluent_this_inventory_no_origin"));
        assert!(!tokens.contains("_this"));
    }

    #[test]
    fn expand_es_fluent_this_returns_compile_errors_for_parse_failures() {
        let this_opts_error: syn::DeriveInput = parse_quote! {
            #[fluent_this(origin = "nope")]
            struct InvalidThisOpts;
        };
        let this_opts_tokens = expand_es_fluent_this(this_opts_error).to_string();
        assert!(this_opts_tokens.contains("compile_error"));

        let struct_namespace_error: syn::DeriveInput = parse_quote! {
            #[fluent_this]
            #[fluent(namespace = 123)]
            struct InvalidStructNamespace;
        };
        let struct_tokens = expand_es_fluent_this(struct_namespace_error).to_string();
        assert!(struct_tokens.contains("compile_error"));

        let enum_namespace_error: syn::DeriveInput = parse_quote! {
            #[fluent_this]
            #[fluent(namespace = 123)]
            enum InvalidEnumNamespace {
                A
            }
        };
        let enum_tokens = expand_es_fluent_this(enum_namespace_error).to_string();
        assert!(enum_tokens.contains("compile_error"));
    }

    #[test]
    fn expand_es_fluent_this_prefers_parent_fluent_namespace_over_this_namespace() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent(namespace = "parent")]
            #[fluent_this(namespace = "child")]
            struct NamespacedThis;
        };

        let tokens = expand_es_fluent_this(input).to_string();
        assert!(tokens.contains("parent"));
        assert!(!tokens.contains("child"));
    }

    #[test]
    fn expand_es_fluent_this_uses_struct_type_kind_for_structs() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_this]
            struct LoginForm;
        };

        let tokens = expand_es_fluent_this(input).to_string();
        assert!(tokens.contains("TypeKind :: Struct"));
        assert!(!tokens.contains("TypeKind :: Enum"));
    }

    #[test]
    fn expand_es_fluent_this_uses_enum_type_kind_for_enums() {
        let input: syn::DeriveInput = parse_quote! {
            #[fluent_this]
            enum LoginState {
                Ready
            }
        };

        let tokens = expand_es_fluent_this(input).to_string();
        assert!(tokens.contains("TypeKind :: Enum"));
    }
}
