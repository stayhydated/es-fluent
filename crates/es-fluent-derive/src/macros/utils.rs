use es_fluent_derive_core::options::namespace::NamespaceValue;
use proc_macro2::TokenStream;
use quote::quote;

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

pub fn namespace_rule_tokens(namespace: Option<&NamespaceValue>) -> TokenStream {
    match namespace {
        Some(NamespaceValue::Literal(s)) => {
            quote! { Some(::es_fluent::registry::NamespaceRule::Literal(#s)) }
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
