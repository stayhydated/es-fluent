//! This module provides types for parsing `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use crate::namer;
use heck::{ToPascalCase as _, ToSnakeCase as _};

pub mod choice;
pub mod r#enum;
pub mod namespace;
pub mod r#struct;
pub mod this;

/// Validate that a key is lowercase snake_case and return its PascalCase version.
///
/// This is a shared helper for `#[fluent_variants]` key validation used by both
/// `EnumVariantsOpts` and `StructVariantsOpts`.
pub fn validate_snake_case_key(key: &syn::LitStr) -> EsFluentCoreResult<String> {
    let key_str = key.value();
    let snake_cased = key_str.to_snake_case();
    let is_lower_snake =
        !key_str.is_empty() && key_str == snake_cased && key_str == key_str.to_ascii_lowercase();

    if !is_lower_snake {
        return Err(EsFluentCoreError::AttributeError {
            message: format!(
                "keys in #[fluent_variants] must be lowercase snake_case; found \"{}\"",
                key_str
            ),
            span: Some(key.span()),
        }
        .with_help("Use values like \"description\" or \"label\".".to_string()));
    }

    Ok(key_str.to_pascal_case())
}

/// Shared behavior for fields that expose Fluent arguments.
pub trait FluentField {
    /// Returns the source field identifier when present.
    fn ident(&self) -> Option<&syn::Ident>;
    /// Returns `true` if the field should be skipped.
    fn is_skipped(&self) -> bool;
    /// Returns `true` if the field is a choice.
    fn is_choice(&self) -> bool;
    /// Returns the value expression if present.
    fn value(&self) -> Option<&syn::Expr>;
    /// Returns explicit field argument name if provided.
    fn arg_name(&self) -> Option<String>;

    /// Resolves the Fluent argument name for this field.
    fn resolved_arg_name(&self, index: usize) -> String {
        self.arg_name()
            .or_else(|| self.ident().map(|ident| ident.to_string()))
            .unwrap_or_else(|| namer::UnnamedItem::from(index).to_string())
    }
}

#[derive(Clone, Debug)]
pub struct ValueAttr(pub syn::Expr);

impl darling::FromMeta for ValueAttr {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::List(list) => {
                let expr: syn::Expr = syn::parse2(list.tokens.clone())?;
                Ok(ValueAttr(expr))
            },
            syn::Meta::NameValue(nv) => {
                // Also support value = "expr" for convenience
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    let expr: syn::Expr = s.parse()?;
                    Ok(ValueAttr(expr))
                } else {
                    Err(darling::Error::unexpected_type("non-string literal"))
                }
            },
            _ => Err(darling::Error::unsupported_format("list or name-value")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::FromMeta as _;

    #[test]
    fn validate_snake_case_key_accepts_and_rejects_expected_values() {
        let good: syn::LitStr = syn::parse_quote!("user_label");
        let converted = validate_snake_case_key(&good).expect("valid snake_case");
        assert_eq!(converted, "UserLabel");

        let bad: syn::LitStr = syn::parse_quote!("UserLabel");
        let err = validate_snake_case_key(&bad).expect_err("invalid key should fail");
        let message = err.to_string();
        assert!(message.contains("lowercase snake_case"));
        assert!(message.contains("help: Use values like"));
    }

    #[test]
    fn value_attr_from_meta_supports_list_and_name_value_string() {
        let list_meta: syn::Meta = syn::parse_quote!(value(|x: &String| x.len()));
        let list = ValueAttr::from_meta(&list_meta).expect("list format");
        let list_expr = list.0;
        assert_eq!(
            quote::quote!(#list_expr).to_string(),
            "| x : & String | x . len ()"
        );

        let nv_meta: syn::Meta = syn::parse_quote!(value = "|x: &str| x.len()");
        let nv = ValueAttr::from_meta(&nv_meta).expect("name-value string");
        let nv_expr = nv.0;
        assert_eq!(
            quote::quote!(#nv_expr).to_string(),
            "| x : & str | x . len ()"
        );
    }

    #[test]
    fn value_attr_from_meta_rejects_non_string_and_unsupported_formats() {
        let non_string_meta: syn::Meta = syn::parse_quote!(value = 123);
        let non_string_err =
            ValueAttr::from_meta(&non_string_meta).expect_err("non-string should fail");
        assert!(!non_string_err.to_string().is_empty());

        let path_meta: syn::Meta = syn::parse_quote!(value);
        let path_err = ValueAttr::from_meta(&path_meta).expect_err("path format should fail");
        assert!(!path_err.to_string().is_empty());
    }
}
