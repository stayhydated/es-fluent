//! Shared validation helpers for macros that do not use `darling` option structs.

use proc_macro2::TokenStream;

use crate::{
    attribute::{AttributeLocation, AttributeName, validate_attribute_for_location},
    error::EsFluentCoreError,
    grammar::LanguageMode,
};

/// Validated inputs for attribute-like and argument-free macros.
pub struct ValidatedMacroInput;

impl ValidatedMacroInput {
    /// Parses and validates `#[es_fluent_language(...)]` arguments.
    pub fn language_mode(attr: TokenStream) -> Result<LanguageMode, EsFluentCoreError> {
        LanguageMode::parse(attr)
    }

    /// Returns true when `attr` is a valid bare `#[locale]` marker for `location`.
    pub fn locale_marker(
        attr: &syn::Attribute,
        location: AttributeLocation,
    ) -> Result<bool, EsFluentCoreError> {
        if !attr.path().is_ident("locale") {
            return Ok(false);
        }

        validate_attribute_for_location(attr, AttributeName::Locale, location, None)?;
        Ok(true)
    }

    /// Rejects function-like macro input for macros that intentionally accept no arguments.
    pub fn reject_argument_free(input: TokenStream, macro_name: &str) -> syn::Result<()> {
        if input.is_empty() {
            return Ok(());
        }

        Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("`{macro_name}` does not accept arguments"),
        ))
    }
}
