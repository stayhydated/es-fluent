use crate::error::{AttrContext, AttrError, EsFluentCoreError, EsFluentCoreResult};
use proc_macro2::Span;
use syn::{Meta, Token, parse::Parser as _, punctuated::Punctuated, spanned::Spanned as _};

use super::{
    AttributeLocation,
    validation::{AttributeSet, LanguageSpec},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageMode {
    Builtin,
    Custom,
}

impl LanguageMode {
    pub fn parse(attr: proc_macro2::TokenStream) -> EsFluentCoreResult<Self> {
        if attr.is_empty() {
            return Ok(Self::Builtin);
        }

        let items = Punctuated::<Meta, Token![,]>::parse_terminated
            .parse2(attr)
            .map_err(|err| {
                language_attr_error(
                    "#[es_fluent_language] expects no arguments, `builtin`, or `custom`",
                    Some(err.span()),
                )
            })?;

        AttributeSet::<LanguageSpec>::validate_items(
            items.iter(),
            AttributeLocation::LanguageContainer,
            None,
        )?;

        if items.is_empty() {
            return Ok(Self::Builtin);
        }

        if let Some(extra) = items.iter().nth(1) {
            return Err(language_attr_error(
                "#[es_fluent_language] accepts at most one mode flag",
                Some(extra.span()),
            ));
        }

        match items.first() {
            Some(Meta::Path(path)) if path.is_ident("builtin") => Ok(Self::Builtin),
            Some(Meta::Path(path)) if path.is_ident("custom") => Ok(Self::Custom),
            other => Err(language_attr_error(
                "#[es_fluent_language] expects no arguments, `builtin`, or `custom`",
                other.map(|meta| meta.span()),
            )),
        }
    }

    pub fn is_custom(self) -> bool {
        matches!(self, Self::Custom)
    }
}

fn language_attr_error(message: impl Into<String>, span: Option<Span>) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        AttrContext::LanguageContainer,
        message,
        span,
    ))
}
