//! This module provides types for naming Fluent keys and documentation.

use derive_more::{Debug, Deref, Display};
use heck::ToSnakeCase as _;
use quote::format_ident;

/// A Fluent key.
#[derive(Clone, Debug, Deref, Display, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct FluentKey(pub String);

impl FluentKey {
    /// The delimiter used in Fluent keys.
    pub const DELIMITER: &str = "-";

    /// Creates a new `FluentKey`.
    pub fn new(ftl_name: &syn::Ident, sub_name: &str) -> Self {
        let normalized_name = ftl_name.to_string().to_snake_case();
        if sub_name.is_empty() {
            FluentKey(normalized_name)
        } else {
            FluentKey(format!(
                "{}{}{}",
                normalized_name,
                Self::DELIMITER,
                sub_name
            ))
        }
    }
}

impl quote::ToTokens for FluentKey {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let key_string = &self.0;
        tokens.extend(quote::quote! { #key_string });
    }
}

/// A Fluent documentation comment.
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

/// An unnamed item.
pub struct UnnamedItem(usize);

impl std::fmt::Display for UnnamedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::UNNAMED_PREFIX, self.0)
    }
}

impl UnnamedItem {
    const UNNAMED_PREFIX: &str = "f";

    /// Converts the unnamed item to an `Ident`.
    pub fn to_ident(&self) -> syn::Ident {
        format_ident!("{}", self.to_string())
    }
}

impl From<usize> for UnnamedItem {
    fn from(index: usize) -> Self {
        Self(index)
    }
}
