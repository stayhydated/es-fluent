use crate::{
    error::{AttrContext, EsFluentCoreResult},
    options::choice::CaseStyle,
};
use es_fluent_shared::fluent::FluentVariantKey;

use super::{FluentChoiceValue, SpannedValue, VariantKey};

/// Semantic mapping for one `EsFluentChoice` enum variant.
#[derive(Clone, Debug)]
pub struct ChoiceVariantModel {
    ident: syn::Ident,
    value: SpannedValue<FluentChoiceValue>,
}

impl ChoiceVariantModel {
    pub fn new(ident: syn::Ident, value: SpannedValue<FluentChoiceValue>) -> Self {
        Self { ident, value }
    }

    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn value(&self) -> &FluentVariantKey {
        self.value.value().variant_key()
    }

    pub fn span(&self) -> proc_macro2::Span {
        self.value.span()
    }
}

/// Input for one selector value generated from an enum variant.
#[derive(Clone, Copy, Debug)]
pub struct ChoiceVariantSource<'a> {
    ident: &'a syn::Ident,
    value_override: Option<&'a SpannedValue<VariantKey>>,
}

impl<'a> ChoiceVariantSource<'a> {
    pub fn new(
        ident: &'a syn::Ident,
        value_override: Option<&'a SpannedValue<VariantKey>>,
    ) -> Self {
        Self {
            ident,
            value_override,
        }
    }
}

/// Semantic model for an `EsFluentChoice` implementation.
#[derive(Clone, Debug)]
pub struct ChoiceModel {
    ident: syn::Ident,
    variants: Vec<ChoiceVariantModel>,
}

impl ChoiceModel {
    pub fn from_variant_idents<'a>(
        ident: &syn::Ident,
        variant_idents: impl IntoIterator<Item = &'a syn::Ident>,
        rename_all: Option<CaseStyle>,
    ) -> EsFluentCoreResult<Self> {
        Self::from_variant_sources(
            ident,
            variant_idents
                .into_iter()
                .map(|variant_ident| ChoiceVariantSource::new(variant_ident, None)),
            rename_all,
        )
    }

    pub fn from_variant_sources<'a>(
        ident: &syn::Ident,
        variant_sources: impl IntoIterator<Item = ChoiceVariantSource<'a>>,
        rename_all: Option<CaseStyle>,
    ) -> EsFluentCoreResult<Self> {
        let rename_all = rename_all.unwrap_or(CaseStyle::KebabCase);
        let variants = variant_sources
            .into_iter()
            .map(|source| {
                let value = if let Some(value_override) = source.value_override {
                    FluentChoiceValue::try_new(
                        value_override.value().as_str().to_string(),
                        value_override.span(),
                        AttrContext::ChoiceContainer,
                    )?
                } else {
                    let variant_name = es_fluent_shared::namer::rust_ident_name(source.ident);
                    let value = rename_all.apply(&variant_name);
                    FluentChoiceValue::try_new(
                        value,
                        source.ident.span(),
                        AttrContext::ChoiceContainer,
                    )?
                };
                Ok(ChoiceVariantModel::new(
                    source.ident.clone(),
                    SpannedValue::new(value, source.ident.span()),
                ))
            })
            .collect::<EsFluentCoreResult<Vec<_>>>()?;

        Ok(Self {
            ident: ident.clone(),
            variants,
        })
    }

    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    pub fn variants(&self) -> &[ChoiceVariantModel] {
        &self.variants
    }
}
