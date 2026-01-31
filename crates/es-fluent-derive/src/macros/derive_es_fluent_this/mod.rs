use darling::FromDeriveInput as _;
use es_fluent_derive_core::{namer, options::this::ThisOpts};
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

        // Generate namespace expression based on attribute
        let namespace_expr = match opts.attr_args().namespace() {
            Some(es_fluent_derive_core::options::namespace::NamespaceValue::Literal(s)) => {
                quote! { Some(#s) }
            },
            Some(es_fluent_derive_core::options::namespace::NamespaceValue::File) => {
                quote! {
                    Some({
                        const FILE_PATH: &str = file!();
                        const NAMESPACE: &str = ::es_fluent::__namespace_from_file_path(FILE_PATH);
                        NAMESPACE
                    })
                }
            },
            Some(es_fluent_derive_core::options::namespace::NamespaceValue::FileRelative) => {
                quote! {
                    Some({
                        const FILE_PATH: &str = file!();
                        const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
                        const NAMESPACE: &str =
                            ::es_fluent::__namespace_from_file_path_relative_with_manifest(
                                FILE_PATH,
                                MANIFEST_DIR,
                            );
                        NAMESPACE
                    })
                }
            },
            None => quote! { None },
        };

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

    tokens.into()
}
