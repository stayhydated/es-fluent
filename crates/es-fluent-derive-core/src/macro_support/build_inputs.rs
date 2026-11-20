use es_fluent_shared::resource::BUILD_MANIFEST_HASH_ENV;
use proc_macro2::TokenStream;
use quote::quote;

/// Returns tokens that make rustc record the package's build manifest hash.
///
/// Macros expand from the manifest `es-fluent-build` writes, which rustc
/// cannot observe. Referencing the manifest's content hash through `env!`
/// lists it in dep-info, so compiler caches key the expansion on it. Returns
/// no tokens when the package has no `es-fluent-build` script.
pub fn build_inputs_tokens() -> TokenStream {
    if std::env::var_os(BUILD_MANIFEST_HASH_ENV).is_none() {
        return TokenStream::new();
    }

    quote! {
        const _: &str = env!(#BUILD_MANIFEST_HASH_ENV);
    }
}

/// Appends [`build_inputs_tokens`] to a macro expansion.
pub fn with_build_inputs(expansion: impl Into<TokenStream>) -> TokenStream {
    let mut expansion = expansion.into();
    expansion.extend(build_inputs_tokens());
    expansion
}
