use crate::error::AttrContext;
use crate::semantic::{
    ArgName, DomainName, FluentMessageId, GeneratedKeyName, SpannedValue, VariantKey,
    parse_arg_name_in_context, parse_domain_name_in_context, parse_fluent_message_id_in_context,
    parse_variant_key_in_context,
};
use darling::FromMeta;

pub(crate) fn string_literal_value(
    item: &syn::Meta,
) -> darling::Result<(String, proc_macro2::Span)> {
    match item {
        syn::Meta::NameValue(name_value) => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) = &name_value.value
            {
                Ok((value.value(), value.span()))
            } else {
                Err(darling::Error::unexpected_type("expected string literal"))
            }
        },
        _ => Err(darling::Error::unsupported_format("string literal")),
    }
}

/// Marker for a bare attribute flag whose grammar accepts only path syntax.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PresentFlag;

impl PresentFlag {
    pub(crate) fn is_present(self) -> bool {
        true
    }
}

impl FromMeta for PresentFlag {
    fn from_word() -> darling::Result<Self> {
        Ok(Self)
    }

    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::Path(_) => Ok(Self),
            _ => Err(darling::Error::custom("use a bare flag").with_span(item)),
        }
    }
}

impl FromMeta for SpannedValue<GeneratedKeyName> {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        let syn::Lit::Str(value) = value else {
            return Err(darling::Error::unexpected_lit_type(value));
        };
        let key =
            GeneratedKeyName::try_new(value.value(), value.span(), AttrContext::VariantsContainer)
                .map_err(|error| darling::Error::custom(error.to_string()).with_span(value))?;
        Ok(SpannedValue::new(key, value.span()))
    }
}

impl FromMeta for SpannedValue<FluentMessageId> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let message_id =
            parse_fluent_message_id_in_context(value, span, AttrContext::MessageContainer)
                .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(message_id, span))
    }
}

impl FromMeta for SpannedValue<ArgName> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let arg = parse_arg_name_in_context(value, span, AttrContext::MessageField)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(arg, span))
    }
}

impl FromMeta for SpannedValue<VariantKey> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let key = parse_variant_key_in_context(value, span, AttrContext::EnumVariant)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(key, span))
    }
}

impl FromMeta for SpannedValue<DomainName> {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        let (value, span) = string_literal_value(item)?;
        let domain = parse_domain_name_in_context(value, span, AttrContext::MessageContainer)
            .map_err(|error| darling::Error::custom(error.to_string()).with_span(item))?;
        Ok(SpannedValue::new(domain, span))
    }
}

#[derive(Clone, Debug)]
pub struct ValueAttr(pub syn::Expr);

impl FromMeta for ValueAttr {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    Err(darling::Error::custom(format!(
                        "expected Rust expression, not string literal; use `value = {}`",
                        s.value()
                    )))
                } else {
                    Ok(ValueAttr(nv.value.clone()))
                }
            },
            _ => Err(darling::Error::unsupported_format(
                "name-value expression, such as `value = |x: &String| x.len()`",
            )),
        }
    }
}
