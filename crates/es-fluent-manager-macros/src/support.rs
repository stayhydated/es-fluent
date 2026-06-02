use es_fluent_derive_core::{
    error::{AttrContext, AttrError, EsFluentCoreError},
    macro_support::ResolvedCratePath,
};
use syn::spanned::Spanned as _;

pub(crate) fn embedded_manager_path() -> ResolvedCratePath {
    ResolvedCratePath::resolve("es-fluent-manager-embedded", "es_fluent_manager_embedded")
}

pub(crate) fn bevy_manager_path() -> ResolvedCratePath {
    ResolvedCratePath::resolve("es-fluent-manager-bevy", "es_fluent_manager_bevy")
}

pub(crate) fn dioxus_manager_path() -> ResolvedCratePath {
    ResolvedCratePath::resolve("es-fluent-manager-dioxus", "es_fluent_manager_dioxus")
}

pub(crate) fn core_error_to_compile_error(error: EsFluentCoreError) -> proc_macro2::TokenStream {
    es_fluent_derive_core::macro_support::core_error_to_compile_error(error)
}

pub(crate) fn validate_locale_marker(attr: &syn::Attribute) -> Result<bool, EsFluentCoreError> {
    if !attr.path().is_ident("locale") {
        return Ok(false);
    }

    match &attr.meta {
        syn::Meta::Path(_) => Ok(true),
        syn::Meta::List(_) => Err(locale_shape_error("#[locale(...)]", attr.span())),
        syn::Meta::NameValue(_) => Err(locale_shape_error("#[locale = ...]", attr.span())),
    }
}

pub(crate) fn unsupported_locale_field_error(
    field: &syn::Field,
    target: &'static str,
) -> EsFluentCoreError {
    let span = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("locale"))
        .map(|attr| attr.span())
        .unwrap_or_else(|| field.span());
    EsFluentCoreError::StructuredAttributeError(AttrError {
        context: AttrContext::LocaleField,
        message: format!("`#[locale]` cannot be used on {target}"),
        span: Some(span),
        note: None,
        help: Some(
            "move #[locale] to a named struct field or named enum variant field".to_string(),
        ),
    })
}

fn locale_shape_error(syntax: &'static str, span: proc_macro2::Span) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError {
        context: AttrContext::LocaleField,
        message: format!("`{syntax}` has the wrong value shape for marker attribute `locale`"),
        span: Some(span),
        note: None,
        help: Some("use #[locale]".to_string()),
    })
}
