use std::str::FromStr as _;

use crate::strategy::DisplayStrategy;
use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use getset::Getters;
use strum::IntoEnumIterator as _;

#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent))]
pub struct EnumFieldOpts {
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    #[getset(get = "pub")]
    ty: syn::Type,
    #[darling(default)]
    skip: Option<bool>,
    #[darling(default)]
    choice: Option<bool>,
}

impl EnumFieldOpts {
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }
}

#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent))]
pub struct VariantOpts {
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<EnumFieldOpts>,
    #[darling(default)]
    skip: Option<bool>,
}

impl VariantOpts {
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
    pub fn style(&self) -> darling::ast::Style {
        self.fields.style
    }
    pub fn fields(&self) -> Vec<&EnumFieldOpts> {
        self.fields
            .iter()
            .filter(|field| !field.is_skipped())
            .collect()
    }
    pub fn all_fields(&self) -> Vec<&EnumFieldOpts> {
        self.fields.iter().collect()
    }
}

#[derive(Clone, FromDeriveInput, Getters)]
#[darling(supports(enum_unit, enum_named, enum_tuple), attributes(fluent))]
#[getset(get = "pub")]
pub struct EnumOpts {
    ident: syn::Ident,
    generics: syn::Generics,
    data: darling::ast::Data<VariantOpts, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumFluentAttributeArgs,
}

impl EnumOpts {
    pub fn variants(&self) -> Vec<&VariantOpts> {
        match &self.data {
            darling::ast::Data::Enum(variants) => variants.iter().collect(),
            _ => unreachable!("Unexpected data type for enum"),
        }
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct EnumFluentAttributeArgs {
    #[darling(default)]
    fl: Option<syn::Path>, // this is probably useless, force user to provide a fl! macro?
    display: Option<String>,
    #[darling(default)]
    choice: Option<bool>,
}

impl EnumFluentAttributeArgs {
    pub fn fl(&self) -> syn::Path {
        let default_fl_path: syn::Path = syn::parse_str(super::DEFAULT_FL_PATH).unwrap();
        self.fl.clone().unwrap_or(default_fl_path)
    }
    pub fn display(&self) -> DisplayStrategy {
        if let Some(mode_str) = self.display.as_deref() {
            DisplayStrategy::from_str(mode_str).unwrap_or_else(|_| {
                let possible_values: Vec<&'static str> = DisplayStrategy::iter()
                    .map(|variant| variant.into())
                    .collect();

                panic!(
                    "Unexpected mode: '{}', expected one of {:?}",
                    mode_str, possible_values
                );
            })
        } else {
            DisplayStrategy::FluentDisplay
        }
    }
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }
}
