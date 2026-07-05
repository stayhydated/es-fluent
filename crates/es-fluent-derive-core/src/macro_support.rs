//! Shared helpers for proc-macro crates built on `es-fluent-derive-core`.

use crate::error::EsFluentCoreError;
use es_fluent_shared::fluent::{
    FluentArgumentName, FluentDomain, FluentMessageId, FluentVariantKey,
};
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};

#[derive(Clone, Debug)]
pub struct ResolvedCratePath {
    tokens: TokenStream,
    rust_path: String,
}

impl ResolvedCratePath {
    pub fn resolve(package_name: &str, fallback_crate_ident: &str) -> Self {
        match crate_name(package_name) {
            Ok(FoundCrate::Itself) => Self {
                tokens: quote! { crate },
                rust_path: "crate".to_string(),
            },
            Ok(FoundCrate::Name(name)) => {
                let ident = format_ident!("{name}");
                Self {
                    tokens: quote! { ::#ident },
                    rust_path: format!("::{name}"),
                }
            },
            Err(_) => Self::fallback(fallback_crate_ident),
        }
    }

    pub fn fallback(crate_ident: &str) -> Self {
        let ident = format_ident!("{crate_ident}");
        Self {
            tokens: quote! { ::#ident },
            rust_path: format!("::{crate_ident}"),
        }
    }

    pub fn tokens(&self) -> &TokenStream {
        &self.tokens
    }

    pub fn rust_path(&self) -> &str {
        &self.rust_path
    }
}

pub fn resolve_crate_path(package_name: &str, fallback_crate_ident: &str) -> TokenStream {
    ResolvedCratePath::resolve(package_name, fallback_crate_ident)
        .tokens()
        .clone()
}

pub fn static_domain_tokens(
    facade_path: &TokenStream,
    domain_override: Option<&FluentDomain>,
) -> TokenStream {
    match domain_override {
        Some(domain) => {
            let domain = domain.as_str();
            quote! { #facade_path::registry::__macro::static_domain(#domain) }
        },
        None => quote! {
            #facade_path::registry::StaticFluentDomain::from_package_name(env!("CARGO_PKG_NAME"))
        },
    }
}

pub fn static_entry_id_tokens(
    facade_path: &TokenStream,
    entry_id: &FluentMessageId,
) -> TokenStream {
    let entry_id = entry_id.as_str();
    quote! {
        #facade_path::registry::__macro::static_entry_id(#entry_id)
    }
}

pub fn static_argument_name_tokens(
    facade_path: &TokenStream,
    argument_name: &FluentArgumentName,
) -> TokenStream {
    let argument_name = argument_name.as_str();
    quote! {
        #facade_path::registry::__macro::static_argument_name(#argument_name)
    }
}

pub fn static_variant_key_tokens(
    facade_path: &TokenStream,
    variant_key: &FluentVariantKey,
) -> TokenStream {
    let variant_key = variant_key.as_str();
    quote! {
        #facade_path::registry::__macro::static_variant_key(#variant_key)
    }
}

pub fn core_error_to_compile_error(error: EsFluentCoreError) -> TokenStream {
    if let EsFluentCoreError::StructuredAttributeErrors(errors) = error {
        let errors = errors.into_iter().map(|error| {
            let message = error.to_string();
            match error.span {
                Some(span) => quote_spanned! { span=> compile_error!(#message); },
                None => quote! { compile_error!(#message); },
            }
        });
        return quote! { #(#errors)* };
    }

    let message = error.to_string();
    match error.span() {
        Some(span) => quote_spanned! { span=> compile_error!(#message); },
        None => quote! { compile_error!(#message); },
    }
}
