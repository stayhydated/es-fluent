//! This module provides types for naming Fluent keys and documentation.

use derive_more::{Debug, Deref, Display};
use heck::ToSnakeCase as _;
use quote::format_ident;

pub fn rust_ident_name(ident: &syn::Ident) -> String {
    let name = ident.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_string()
}

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
        Self(rust_ident_name(ident).to_snake_case())
    }
}

impl FluentKey {
    pub const DELIMITER: &str = "-";
    pub const LABEL_SUFFIX: &str = "_label";

    pub fn join(&self, suffix: impl std::fmt::Display) -> Self {
        let suffix_str = suffix.to_string();
        if suffix_str.is_empty() {
            self.clone()
        } else {
            Self(format!("{}{}{}", self.0, Self::DELIMITER, suffix_str))
        }
    }

    pub fn new_label(ftl_name: &syn::Ident) -> Self {
        let label_ident =
            quote::format_ident!("{}{}", rust_ident_name(ftl_name), Self::LABEL_SUFFIX);
        Self::from(&label_ident)
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn fluent_key_conversions_and_joining_work() {
        let from_string = FluentKey::from("hello_world".to_string());
        let from_str = FluentKey::from("hello_world");
        let from_ident = FluentKey::from(&syn::Ident::new(
            "HelloWorld",
            proc_macro2::Span::call_site(),
        ));
        let raw_ident: syn::Ident = syn::parse_str("r#type").expect("raw ident");
        let from_raw_ident = FluentKey::from(&raw_ident);

        assert_eq!(from_string.to_string(), "hello_world");
        assert_eq!(from_str.to_string(), "hello_world");
        assert_eq!(from_ident.to_string(), "hello_world");
        assert_eq!(rust_ident_name(&raw_ident), "type");
        assert_eq!(from_raw_ident.to_string(), "type");

        assert_eq!(from_ident.join("suffix").to_string(), "hello_world-suffix");
        assert_eq!(from_ident.join("").to_string(), "hello_world");
    }

    #[test]
    fn fluent_key_label_and_token_generation_work() {
        let label_key =
            FluentKey::new_label(&syn::Ident::new("MyType", proc_macro2::Span::call_site()));
        assert_eq!(label_key.to_string(), "my_type_label");

        let tokens = quote!(#label_key).to_string();
        assert!(tokens.contains("my_type_label"));
    }

    #[test]
    fn fluent_doc_and_unnamed_item_cover_display_and_tokens() {
        let key = FluentKey::from("field_name");
        let doc = FluentDoc::from(&key);
        let doc_tokens = quote!(#doc).to_string();
        assert!(doc_tokens.contains("Key ="));
        assert!(doc_tokens.contains("field_name"));

        let unnamed = UnnamedItem::from(3);
        assert_eq!(unnamed.to_string(), "f3");
        assert_eq!(unnamed.to_ident().to_string(), "f3");
    }
}
