use crate::error::{AttrContext, AttrError, ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
use es_fluent_shared::{
    fluent::{
        FluentArgumentName, FluentDomain, FluentIdentifierError,
        FluentMessageId as SharedMessageId, FluentVariantKey,
    },
    namer,
};
use heck::{ToPascalCase as _, ToSnakeCase as _};
use proc_macro2::Span;

use super::{ArgName, DomainName, FluentMessageId, VariantKey};

/// A value paired with the best source span available for diagnostics or code emission.
#[derive(Clone, Debug)]
pub struct SpannedValue<T> {
    value: T,
    span: Span,
}

impl<T> SpannedValue<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn into_value(self) -> T {
        self.value
    }
}

pub fn parse_arg_name(value: impl Into<String>, span: Span) -> EsFluentCoreResult<ArgName> {
    parse_arg_name_in_context(value, span, AttrContext::MessageContainer)
}

pub fn parse_arg_name_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<ArgName> {
    FluentArgumentName::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn parse_variant_key_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<VariantKey> {
    FluentVariantKey::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn parse_domain_name_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<DomainName> {
    FluentDomain::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn parse_fluent_message_id_in_context(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<FluentMessageId> {
    SharedMessageId::try_new(value).map_err(|error| semantic_error(error, span, context))
}

pub fn spanned_message_id_from_value(
    value: impl Into<String>,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    let value = parse_fluent_message_id_in_context(value, span, context)?;
    Ok(SpannedValue::new(value, span))
}

pub fn message_id_from_fluent_key(
    key: namer::FluentKey,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    spanned_message_id_from_value(key.to_string(), span, context)
}

pub fn message_id_for_ident(
    ident: &syn::Ident,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    message_id_from_fluent_key(namer::FluentKey::from(ident), ident.span(), context)
}

pub fn label_message_id_for_ident(
    ident: &syn::Ident,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    message_id_from_fluent_key(namer::FluentKey::new_label(ident), ident.span(), context)
}

pub fn variant_message_id(
    base_key: &FluentMessageId,
    variant_ident: &syn::Ident,
    override_key: Option<&VariantKey>,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    let variant_key_suffix = override_key
        .map(VariantKey::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| namer::rust_ident_name(variant_ident));
    message_id_from_fluent_key(
        namer::FluentKey::from(base_key.as_str()).join(&variant_key_suffix),
        variant_ident.span(),
        context,
    )
}

pub fn generated_variant_message_id(
    base_key: &namer::FluentKey,
    key_fragment: &str,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    message_id_from_fluent_key(base_key.join(key_fragment), span, context)
}

pub fn generated_label_message_id(
    base_key: &namer::FluentKey,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<SpannedValue<FluentMessageId>> {
    spanned_message_id_from_value(
        format!("{}{}", base_key, namer::FluentKey::LABEL_SUFFIX),
        span,
        context,
    )
}

pub fn generated_label_message_value(
    base_key: &namer::FluentKey,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreResult<FluentMessageId> {
    generated_label_message_id(base_key, span, context).map(SpannedValue::into_value)
}

/// Rust type identifier metadata preserved with its source span.
#[derive(Clone, Debug)]
pub struct RustTypeName {
    value: String,
    span: Span,
}

impl RustTypeName {
    pub fn from_ident(ident: &syn::Ident) -> Self {
        Self {
            value: namer::rust_ident_name(ident),
            span: ident.span(),
        }
    }

    pub fn new(value: impl Into<String>, span: Span) -> Self {
        Self {
            value: value.into(),
            span,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

/// Rust item name metadata preserved with its source span.
#[derive(Clone, Debug)]
pub struct RustSourceName {
    value: String,
    span: Span,
}

impl RustSourceName {
    pub fn from_ident(ident: &syn::Ident) -> Self {
        Self {
            value: namer::rust_ident_name(ident),
            span: ident.span(),
        }
    }

    pub fn new(value: impl Into<String>, span: Span) -> Self {
        Self {
            value: value.into(),
            span,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

/// A typed generated variant key from `#[fluent_variants(keys = [...])]`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GeneratedKeyName {
    value: String,
}

impl GeneratedKeyName {
    pub fn try_new(
        value: impl Into<String>,
        span: Span,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        let value = value.into();
        let snake_cased = value.to_snake_case();
        let is_lower_snake =
            !value.is_empty() && value == snake_cased && value == value.to_ascii_lowercase();

        if !is_lower_snake {
            return Err(EsFluentCoreError::StructuredAttributeError(AttrError::new(
                context,
                format!(
                    "keys in #[fluent_variants] must be lowercase snake_case; found \"{}\"",
                    value
                ),
                Some(span),
            ))
            .with_help("use values like \"description\" or \"label\"".to_string()));
        }

        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn to_pascal_case(&self) -> String {
        self.value.to_pascal_case()
    }
}

/// A generated Rust identifier derived from a typed generated variant key.
#[derive(Clone, Debug)]
pub struct GeneratedKeyIdent {
    ident: syn::Ident,
}

impl GeneratedKeyIdent {
    pub fn variants(
        source_ident: &syn::Ident,
        key: &SpannedValue<GeneratedKeyName>,
        suffix: &str,
    ) -> Self {
        Self::from_parts(source_ident, key, suffix)
    }

    pub fn base(source_ident: &syn::Ident, key: &SpannedValue<GeneratedKeyName>) -> Self {
        Self::from_parts(source_ident, key, "")
    }

    fn from_parts(
        source_ident: &syn::Ident,
        key: &SpannedValue<GeneratedKeyName>,
        suffix: &str,
    ) -> Self {
        let ident = syn::Ident::new(
            &format!(
                "{}{}{}",
                namer::rust_ident_name(source_ident),
                key.value().to_pascal_case(),
                suffix
            ),
            key.span(),
        );
        Self { ident }
    }

    pub fn into_ident(self) -> syn::Ident {
        self.ident
    }
}

/// Display/source name used in generated enum documentation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedDocName(String);

impl GeneratedDocName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for GeneratedDocName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// A Fluent selector value emitted by `EsFluentChoice`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluentChoiceValue {
    value: FluentVariantKey,
}

impl FluentChoiceValue {
    pub fn try_new(
        value: impl Into<String>,
        span: Span,
        context: AttrContext,
    ) -> EsFluentCoreResult<Self> {
        Ok(Self {
            value: parse_variant_key_in_context(value, span, context)?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn variant_key(&self) -> &FluentVariantKey {
        &self.value
    }
}

/// Source location metadata for a generated semantic model entry.
#[derive(Clone, Debug)]
pub struct SourceLocation {
    span: Span,
}

impl SourceLocation {
    pub fn new(span: Span) -> Self {
        Self { span }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

fn semantic_error(
    error: FluentIdentifierError,
    span: Span,
    context: AttrContext,
) -> EsFluentCoreError {
    EsFluentCoreError::StructuredAttributeError(AttrError::new(
        context,
        error.to_string(),
        Some(span),
    ))
}
