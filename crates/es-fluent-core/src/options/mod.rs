//! This module provides types for parsing `es-fluent` attributes.

pub mod r#enum;
pub mod r#struct;
pub mod this;

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
