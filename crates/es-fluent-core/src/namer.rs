use derive_more::{Debug, Deref, Display};
use heck::ToSnakeCase as _;
use quote::format_ident;

#[derive(Clone, Debug, Deref, Display, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FluentKey(pub String);

impl FluentKey {
    pub const DELIMITER: &str = "-";
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
    use proc_macro2::Span;

    #[test]
    fn test_fluent_key_new_with_subname() {
        let ident = syn::Ident::new("TestName", Span::call_site());
        let key = FluentKey::new(&ident, "sub_part");
        assert_eq!(key.0, "test_name-sub_part");
    }

    #[test]
    fn test_fluent_key_new_empty_subname() {
        let ident = syn::Ident::new("TestName", Span::call_site());
        let key = FluentKey::new(&ident, "");
        assert_eq!(key.0, "test_name");
    }

    #[test]
    fn test_fluent_doc_from_fluent_key() {
        let key = FluentKey("test_key".to_string());
        let doc: FluentDoc = (&key).into();
        assert_eq!(doc.to_string(), "Key = `test_key`");
    }

    #[test]
    fn test_unnamed_item_display() {
        let item = UnnamedItem(5);
        assert_eq!(item.to_string(), "f5");
    }

    #[test]
    fn test_unnamed_item_from_usize() {
        let item: UnnamedItem = 10.into();
        assert_eq!(item.to_string(), "f10");
    }

    #[test]
    fn test_unnamed_item_to_ident() {
        let item = UnnamedItem(0);
        let ident = item.to_ident();
        assert_eq!(ident.to_string(), "f0");
    }
}
