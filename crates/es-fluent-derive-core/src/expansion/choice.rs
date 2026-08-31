use super::*;

/// Validated data needed to emit an `EsFluentChoice` implementation.
#[derive(Clone, Debug)]
pub struct EsFluentChoiceExpansion {
    ident: syn::Ident,
    generics: syn::Generics,
    choice: ChoiceModel,
}

impl EsFluentChoiceExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let input = ValidatedDeriveInput::for_es_fluent_choice(input)?;
        let input = input.input();

        let opts = ChoiceOpts::from_derive_input(input)?;
        let lowered = lowered::ChoiceModel::from_options(&opts)?;
        let enum_ident = lowered.ident();
        let choice = ChoiceModel::from_variant_idents(
            enum_ident,
            lowered.variants().iter().map(|variant| variant.ident()),
            *opts.attr_args().rename_all(),
        )?;

        Ok(Self {
            ident: enum_ident.clone(),
            generics: opts.generics().clone(),
            choice,
        })
    }

    /// The enum identifier receiving the generated implementation.
    pub fn ident(&self) -> &syn::Ident {
        &self.ident
    }

    /// The enum generics preserved from the user-authored type.
    pub fn generics(&self) -> &syn::Generics {
        &self.generics
    }

    /// The final variant-to-choice-value mapping.
    pub fn choice(&self) -> &ChoiceModel {
        &self.choice
    }
}
