//! This module provides types for naming Fluent keys and documentation.

use derive_more::{Debug, Deref, Display};
use heck::ToSnakeCase as _;
use quote::format_ident;

#[derive(Clone, Debug, Deref, Display, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct FluentKey(pub String);

impl FluentKey {
    pub const DELIMITER: &str = "-";

    pub fn new(ftl_name: &syn::Ident, sub_name: &str) -> Self {
        let normalized_name = ftl_name.to_string().to_snake_case();
        Self::with_base(&normalized_name, sub_name)
    }

    pub fn with_base(base: &str, sub_name: &str) -> Self {
        if sub_name.is_empty() {
            FluentKey(base.to_string())
        } else {
            FluentKey(format!("{}{}{}", base, Self::DELIMITER, sub_name))
        }
    }

    pub fn this(ftl_name: &syn::Ident) -> Self {
        let this_ident = quote::format_ident!("{}_this", ftl_name);
        Self::new(&this_ident, "")
    }

    pub fn this_from_base(base: &str) -> Self {
        let this_base = format!("{}_this", base);
        Self::with_base(&this_base, "")
    }
}

impl quote::ToTokens for FluentKey {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let key_string = &self.0;
        tokens.extend(quote::quote! { #key_string });
    }
}

#[derive(Clone, Debug, Deref, Display, Eq, Hash, PartialEq)]
pub struct FluentDoc(String);

impl From<&FluentKey> for FluentDoc {
    fn from(ftl_key: &FluentKey) -> Self {
        FluentDoc(format!("Key = `{}`", *ftl_key))
    }
}

impl quote::ToTokens for FluentDoc {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let doc_string = &self.0;
        tokens.extend(quote::quote! { #doc_string });
    }
}

pub struct UnnamedItem(usize);

impl std::fmt::Display for UnnamedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::UNNAMED_PREFIX, self.0)
    }
}

impl UnnamedItem {
    const UNNAMED_PREFIX: &str = "f";

    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("{}", self.to_string())
    }
}

impl From<usize> for UnnamedItem {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
