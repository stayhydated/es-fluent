use es_fluent_shared::resource::BUILD_INPUTS_FINGERPRINT_ENV;
use proc_macro2::TokenStream;
use quote::quote;

/// Returns tokens that make rustc record the package's i18n input fingerprint.
///
/// Macros read `i18n.toml`, locale directories, and the fallback catalog from
/// disk, which rustc cannot observe. `es-fluent-build` fingerprints those
/// inputs into an environment variable; referencing it through `env!` lists it
/// in dep-info, so compiler caches key the expansion on it. Returns no tokens
/// when the package has no `es-fluent-build` script.
pub fn build_inputs_tokens() -> TokenStream {
    if std::env::var_os(BUILD_INPUTS_FINGERPRINT_ENV).is_none() {
        return TokenStream::new();
    }

    quote! {
        const _: &str = env!(#BUILD_INPUTS_FINGERPRINT_ENV);
    }
}

/// Appends [`build_inputs_tokens`] to a macro expansion.
pub fn with_build_inputs(expansion: impl Into<TokenStream>) -> TokenStream {
    let mut expansion = expansion.into();
    expansion.extend(build_inputs_tokens());
    expansion
}
