use darling::FromDeriveInput as _;
use es_fluent_core::{namer, options::this::ThisOpts};
use heck::ToSnakeCase as _;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let opts = match ThisOpts::from_derive_input(&input) {
        Ok(opts) => opts,
        Err(err) => return err.write_errors().into(),
    };

    let original_ident = opts.ident();
    let generics = opts.generics();
    let ftl_key = if opts.attr_args().is_origin() {
        let this_ident = quote::format_ident!("{}_this", original_ident);
        Some(namer::FluentKey::new(&this_ident, "").to_string())
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

        quote! {
            #[doc(hidden)]
            mod #mod_name {
                use super::*;

                static VARIANTS: &[::es_fluent::__core::registry::StaticFtlVariant] = &[
                    ::es_fluent::__core::registry::StaticFtlVariant {
                        name: #type_name,
                        ftl_key: #ftl_key_str,
                        args: &[],
                        is_this: true,
                    }
                ];

                static TYPE_INFO: ::es_fluent::__core::registry::StaticFtlTypeInfo =
                    ::es_fluent::__core::registry::StaticFtlTypeInfo {
                        type_kind: ::es_fluent::__core::meta::TypeKind::Enum,
                        type_name: #type_name,
                        variants: VARIANTS,
                        file_path: file!(),
                        is_this: true,
                    };

                ::es_fluent::__inventory::submit!(&TYPE_INFO);
            }
        }
    } else {
        quote! {}
    };

    let tokens = quote! {
        #this_ftl_impl
        #inventory_submit
    };

    tokens.into()
}
