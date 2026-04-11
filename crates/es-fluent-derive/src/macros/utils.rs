use es_fluent_derive_core::options::namespace::NamespaceValue;
use heck::ToSnakeCase as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub struct InventoryModuleInput<'a> {
    pub ident: &'a syn::Ident,
    pub module_name_prefix: &'a str,
    pub type_kind: TokenStream,
    pub variants: Vec<TokenStream>,
    pub namespace_expr: TokenStream,
}

/// Generates the `ThisFtl` trait implementation.
pub fn generate_this_ftl_impl(
    ident: &syn::Ident,
    generics: &syn::Generics,
    ftl_key: Option<&str>,
) -> TokenStream {
    let Some(ftl_key) = ftl_key else {
        return quote! {};
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        impl #impl_generics ::es_fluent::ThisFtl for #ident #ty_generics #where_clause {
            fn this_ftl() -> String {
                ::es_fluent::localize(#ftl_key, None)
            }
        }
    }
}

pub fn generate_field_value_expr(
    access_expr: TokenStream,
    transform_arg_expr: TokenStream,
    value_expr: Option<&syn::Expr>,
    is_choice: bool,
) -> TokenStream {
    if let Some(expr) = value_expr {
        quote! { (#expr)(#transform_arg_expr) }
    } else if is_choice {
        quote! { { use ::es_fluent::EsFluentChoice as _; (#access_expr).as_fluent_choice() } }
    } else {
        quote! { (#access_expr).clone() }
    }
}

pub fn generate_from_impls(ident: &syn::Ident, generics: &syn::Generics) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics From<&#ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: &#ident #ty_generics) -> Self {
                use ::es_fluent::ToFluentString as _;
                value.to_fluent_string().into()
            }
        }

        impl #impl_generics From<#ident #ty_generics> for ::es_fluent::FluentValue<'_> #where_clause {
            fn from(value: #ident #ty_generics) -> Self {
                (&value).into()
            }
        }
    }
}

pub fn generate_inventory_module(input: InventoryModuleInput<'_>) -> TokenStream {
    let InventoryModuleInput {
        ident,
        module_name_prefix,
        type_kind,
        variants,
        namespace_expr,
    } = input;

    let type_name = ident.to_string();
    let mod_name = format_ident!(
        "__es_fluent_{}_{}",
        module_name_prefix,
        type_name.to_snake_case()
    );

    quote! {
        #[doc(hidden)]
        mod #mod_name {
            use super::*;

            static VARIANTS: &[::es_fluent::registry::FtlVariant] = &[
                #(#variants),*
            ];

            static TYPE_INFO: ::es_fluent::registry::FtlTypeInfo =
                ::es_fluent::registry::FtlTypeInfo {
                    type_kind: #type_kind,
                    type_name: #type_name,
                    variants: VARIANTS,
                    file_path: file!(),
                    module_path: module_path!(),
                    namespace: #namespace_expr,
                };

            ::es_fluent::__inventory::submit!(::es_fluent::registry::RegisteredFtlType(&TYPE_INFO));
        }
    }
}

pub fn namespace_rule_tokens(namespace: Option<&NamespaceValue>) -> TokenStream {
    match namespace {
        Some(NamespaceValue::Literal(s)) => {
            quote! {
                Some(::es_fluent::registry::NamespaceRule::Literal(
                    ::std::borrow::Cow::Borrowed(#s)
                ))
            }
        },
        Some(NamespaceValue::File) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::File) }
        },
        Some(NamespaceValue::FileRelative) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::FileRelative) }
        },
        Some(NamespaceValue::Folder) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::Folder) }
        },
        Some(NamespaceValue::FolderRelative) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::FolderRelative) }
        },
        None => quote! { None },
    }
}
