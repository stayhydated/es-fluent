use darling::FromDeriveInput as _;
use es_fluent_derive_core::{
    namer,
    options::{r#enum::EnumOpts, r#struct::StructOpts, this::ThisOpts},
};
use heck::ToSnakeCase as _;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

use crate::macros::utils::namespace_rule_tokens;

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_es_fluent_this(input).into()
}

fn expand_es_fluent_this(input: DeriveInput) -> proc_macro2::TokenStream {
    let opts = match ThisOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors(),
    };

    let fluent_namespace = match &input.data {
        Data::Struct(_) => match StructOpts::from_derive_input(&input) {
            Ok(opts) => opts.attr_args().namespace().cloned(),
            Err(err) => return err.write_errors(),
        },
        Data::Enum(_) => match EnumOpts::from_derive_input(&input) {
            Ok(opts) => opts.attr_args().namespace().cloned(),
            Err(err) => return err.write_errors(),
        },
        Data::Union(_) => panic!("EsFluentThis cannot be used on unions"),
    };

    let original_ident = opts.ident();
    let generics = opts.generics();
    let ftl_key = if opts.attr_args().is_origin() {
        Some(namer::FluentKey::new_this(original_ident).to_string())
    } else {
        None
    };

    let this_ftl_impl =
        crate::macros::utils::generate_this_ftl_impl(original_ident, generics, ftl_key.as_deref());

    // Generate inventory submission for types with origin=true
    // FTL metadata is purely structural and doesn't depend on generic type parameters
    let inventory_submit = if let Some(ftl_key_str) = &ftl_key {
        let type_name = original_ident.to_string();
        let mod_name =
            quote::format_ident!("__es_fluent_this_inventory_{}", type_name.to_snake_case());

        let namespace_expr = namespace_rule_tokens(
            fluent_namespace
                .as_ref()
                .or_else(|| opts.attr_args().namespace()),
        );

        quote! {
            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                static VARIANTS: &[::es_fluent::registry::FtlVariant] = &[
                    ::es_fluent::registry::FtlVariant {
                        name: #type_name,
                        ftl_key: #ftl_key_str,
                        args: &[],
                        module_path: module_path!(),
                        line: line!(),
                    }
                ];

                static TYPE_INFO: ::es_fluent::registry::FtlTypeInfo =
                    ::es_fluent::registry::FtlTypeInfo {
                        type_kind: ::es_fluent::meta::TypeKind::Enum,
                        type_name: #type_name,
                        variants: VARIANTS,
                        file_path: file!(),
                        module_path: module_path!(),
                        namespace: #namespace_expr,
                    };

                ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
            }
        }
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
}
