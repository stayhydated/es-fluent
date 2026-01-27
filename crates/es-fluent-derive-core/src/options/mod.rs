//! This module provides types for parsing `es-fluent` attributes.

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use heck::{ToPascalCase as _, ToSnakeCase as _};

pub mod choice;
pub mod r#enum;
pub mod namespace;
pub mod r#struct;
pub mod this;

/// Validate that a key is lowercase snake_case and return its PascalCase version.
///
/// This is a shared helper for `#[fluent_variants]` key validation used by both
/// `EnumKvOpts` and `StructKvOpts`.
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
