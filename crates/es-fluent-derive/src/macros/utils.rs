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
