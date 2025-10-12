use std::str::FromStr as _;

use crate::strategy::DisplayStrategy;
use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use getset::Getters;
use strum::IntoEnumIterator as _;

/// Options for an enum field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent))]
pub struct EnumFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
    /// Whether this field is a choice.
    #[darling(default)]
    choice: Option<bool>,
}

impl EnumFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
    /// Returns `true` if the field is a choice.
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }
}

/// Options for an enum variant.
#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent))]
pub struct VariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<EnumFieldOpts>,
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<bool>,
}

impl VariantOpts {
    /// Returns `true` if the variant should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
    /// Returns the style of the variant's fields.
    pub fn style(&self) -> darling::ast::Style {
        self.fields.style
    }
    /// Returns the fields of the variant that are not skipped.
    pub fn fields(&self) -> Vec<&EnumFieldOpts> {
        self.fields
            .iter()
            .filter(|field| !field.is_skipped())
            .collect()
    }
    /// Returns all fields of the variant.
    pub fn all_fields(&self) -> Vec<&EnumFieldOpts> {
        self.fields.iter().collect()
    }
}

/// Options for an enum.
#[derive(Clone, FromDeriveInput, Getters, Debug)]
#[darling(supports(enum_unit, enum_named, enum_tuple), attributes(fluent))]
#[getset(get = "pub")]
pub struct EnumOpts {
    /// The identifier of the enum.
    ident: syn::Ident,
    /// The generics of the enum.
    generics: syn::Generics,
    data: darling::ast::Data<VariantOpts, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumFluentAttributeArgs,
}

impl EnumOpts {
    /// Returns the variants of the enum.
    pub fn variants(&self) -> Vec<&VariantOpts> {
        match &self.data {
            darling::ast::Data::Enum(variants) => variants.iter().collect(),
            _ => unreachable!("Unexpected data type for enum"),
        }
    }
}

/// Attribute arguments for an enum.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct EnumFluentAttributeArgs {
    display: Option<String>,
    #[darling(default)]
    choice: Option<bool>,
    #[darling(default)]
    this: Option<bool>,
}

impl EnumFluentAttributeArgs {
    /// Returns the display strategy for the enum.
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
    /// Returns `true` if the enum is a choice.
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }
    /// Returns `true` if the enum should be passed as `this`.
    pub fn is_this(&self) -> bool {
        self.this.unwrap_or(false)
    }
}

/// Options for an enum that can be used as a choice.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(enum_unit), attributes(fluent_choice))]
#[getset(get = "pub")]
pub struct EnumChoiceOpts {
    /// The identifier of the enum.
    ident: syn::Ident,
    /// The generics of the enum.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumChoiceAttributeArgs,
}

/// Attribute arguments for an enum that can be used as a choice.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
#[getset(get = "pub")]
pub struct EnumChoiceAttributeArgs {
    #[darling(default)]
    serialize_all: Option<String>,
}
