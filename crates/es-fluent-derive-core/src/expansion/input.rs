use super::*;

/// Errors that can occur while building a derive expansion model.
#[derive(Debug, thiserror::Error)]
pub enum ExpansionError {
    /// A structured derive-core validation error.
    #[error(transparent)]
    Core(#[from] EsFluentCoreError),
    /// A `darling` option parsing error.
    #[error(transparent)]
    Darling(#[from] darling::Error),
    /// A `syn` parse or input-shape error.
    #[error(transparent)]
    Syn(#[from] syn::Error),
}

/// A result type for expansion model construction.
pub type ExpansionResult<T> = Result<T, ExpansionError>;

/// Derive surface whose raw input has been grammar-validated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeriveFamily {
    EsFluent,
    EsFluentLabel,
    EsFluentVariants,
    EsFluentChoice,
}

/// A derive input after the raw attribute grammar has been validated once.
#[derive(Clone, Debug)]
pub struct ValidatedDeriveInput<'a> {
    input: &'a syn::DeriveInput,
    family: DeriveFamily,
    envelope: Option<ContainerEnvelope>,
}

impl<'a> ValidatedDeriveInput<'a> {
    pub fn for_es_fluent(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        derive_validation::validate_es_fluent_attribute_context(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluent,
            envelope: None,
        })
    }

    pub fn for_es_fluent_label(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        derive_validation::validate_es_fluent_label_attribute_context(input)?;
        let envelope = ContainerEnvelope::from_derive_input(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluentLabel,
            envelope: Some(envelope),
        })
    }

    pub fn for_es_fluent_variants(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        if matches!(&input.data, Data::Union(_)) {
            return Err(syn::Error::new(
                input.ident.span(),
                "EsFluentVariants can only be derived for structs and enums",
            )
            .into());
        }

        derive_validation::validate_es_fluent_variants_attribute_context(input)?;
        let envelope = ContainerEnvelope::from_derive_input(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluentVariants,
            envelope: Some(envelope),
        })
    }

    pub fn for_es_fluent_choice(input: &'a syn::DeriveInput) -> ExpansionResult<Self> {
        derive_validation::validate_es_fluent_choice_attribute_context(input)?;
        Ok(Self {
            input,
            family: DeriveFamily::EsFluentChoice,
            envelope: None,
        })
    }

    pub fn input(&self) -> &'a syn::DeriveInput {
        self.input
    }

    pub fn family(&self) -> DeriveFamily {
        self.family
    }

    pub fn envelope(&self) -> Option<&ContainerEnvelope> {
        self.envelope.as_ref()
    }

    pub(super) fn required_envelope(&self) -> ExpansionResult<&ContainerEnvelope> {
        self.envelope.as_ref().ok_or_else(|| {
            syn::Error::new(
                self.input.ident.span(),
                "internal error: validated derive input is missing container context",
            )
            .into()
        })
    }
}
