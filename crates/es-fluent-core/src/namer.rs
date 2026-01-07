//! This module provides types for naming Fluent keys and documentation.

use derive_more::{Debug, Deref, Display};
use heck::ToSnakeCase as _;
use quote::format_ident;

#[derive(Clone, Debug, Deref, Display, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct FluentKey(pub String);

impl From<String> for FluentKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for FluentKey {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

impl From<&syn::Ident> for FluentKey {
    fn from(ident: &syn::Ident) -> Self {
        Self(ident.to_string().to_snake_case())
    }
}

impl FluentKey {
    pub const DELIMITER: &str = "-";
    pub const THIS_SUFFIX: &str = "_this";

    pub fn join(&self, suffix: impl std::fmt::Display) -> Self {
        let suffix_str = suffix.to_string();
        if suffix_str.is_empty() {
            self.clone()
        } else {
            Self(format!("{}{}{}", self.0, Self::DELIMITER, suffix_str))
        }
    }

    pub fn new_this(ftl_name: &syn::Ident) -> Self {
        let this_ident = quote::format_ident!("{}{}", ftl_name, Self::THIS_SUFFIX);
        Self::from(&this_ident)
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
