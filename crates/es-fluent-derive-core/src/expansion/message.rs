use super::*;

/// Validated data needed to emit an `EsFluent` implementation.
#[derive(Clone, Debug)]
pub enum EsFluentExpansion {
    /// Struct implementation data.
    Struct(EsFluentStructExpansion),
    /// Enum implementation data.
    Enum(EsFluentEnumExpansion),
}

impl EsFluentExpansion {
    /// Builds a validated expansion model from the user's derive input.
    pub fn from_derive_input(input: &syn::DeriveInput) -> ExpansionResult<Self> {
        let input = ValidatedDeriveInput::for_es_fluent(input)?;
        let input = input.input();

        match &input.data {
            Data::Struct(_) => {
                let opts = StructOpts::from_derive_input(input)?;
                Ok(Self::Struct(EsFluentStructExpansion::from_options(&opts)?))
            },
            Data::Enum(_) => {
                let opts = EnumOpts::from_derive_input(input)?;
                let choice_config = message_enum::inferred_choice_config(input)?;
                Ok(Self::Enum(EsFluentEnumExpansion::from_options_with_choice(
                    &opts,
                    choice_config,
                )?))
            },
            Data::Union(_) => Err(syn::Error::new(
                input.ident.span(),
                "EsFluent can only be derived for structs and enums",
            )
            .into()),
        }
    }
}
