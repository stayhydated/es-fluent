use std::str::FromStr as _;

use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta};
use getset::Getters;
use quote::format_ident;
use strum::IntoEnumIterator as _;

use crate::strategy::DisplayStrategy;

/// Options for a struct field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent))]
pub struct StructFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
    /// Whether this field is a default.
    #[darling(default)]
    default: Option<bool>,
    /// Whether this field is a choice.
    #[darling(default)]
    choice: Option<bool>,
}

impl StructFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    /// Returns `true` if the field is a default.
    pub fn is_default(&self) -> bool {
        self.default.unwrap_or(false)
    }

    /// Returns `true` if the field is a choice.
    pub fn is_choice(&self) -> bool {
        self.choice.unwrap_or(false)
    }
}

/// Options for a struct.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_named), attributes(fluent))]
#[getset(get = "pub")]
pub struct StructOpts {
    /// The identifier of the struct.
    ident: syn::Ident,
    /// The generics of the struct.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, StructFieldOpts>,
    #[darling(flatten)]
    attr_args: StructFluentAttributeArgs,
}

impl StructOpts {
    /// Returns the fields of the struct that are not skipped.
    pub fn fields(&self) -> Vec<&StructFieldOpts> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields
                .fields
                .iter()
                .filter(|field| !field.is_skipped())
                .collect(),
            _ => vec![],
        }
    }
}

/// Attribute arguments for a struct.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructFluentAttributeArgs {
    display: Option<String>,
    #[darling(default)]
    this: Option<bool>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
}
impl StructFluentAttributeArgs {
    /// Returns the display strategy for the struct.
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
    /// Returns `true` if the struct should be passed as `this`.
    pub fn is_this(&self) -> bool {
        self.this.unwrap_or(false)
    }
}

/// Options for a struct field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent_kv))]
pub struct StructKvFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
    /// Whether this field is a default.
    #[darling(default)]
    default: Option<bool>,
}

impl StructKvFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    /// Returns `true` if the field is a default.
    pub fn is_default(&self) -> bool {
        self.default.unwrap_or(false)
    }
}

/// Options for a struct.
#[derive(Clone, FromDeriveInput, Getters, Debug)]
#[darling(supports(struct_named), attributes(fluent_kv))]
#[getset(get = "pub")]
pub struct StructKvOpts {
    /// The identifier of the struct.
    ident: syn::Ident,
    /// The generics of the struct.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, StructKvFieldOpts>,
    #[darling(flatten)]
    attr_args: StructKvFluentAttributeArgs,
}

impl StructKvOpts {
    const FTL_ENUM_IDENT: &str = "Ftl";

    /// Returns the identifier of the FTL enum.
    pub fn ftl_enum_ident(&self) -> syn::Ident {
        format_ident!("{}{}", &self.ident, Self::FTL_ENUM_IDENT)
    }

    /// Returns the identifiers of the keyed FTL enums.
    pub fn keyyed_idents(&self) -> Vec<syn::Ident> {
        self.attr_args
            .clone()
            .keys
            .map_or_else(Vec::new, |keys| {
                keys.into_iter()
                    .map(|key| format_ident!("{}", key.value()))
                    .collect()
            })
            .into_iter()
            .map(|key| format_ident!("{}{}{}", &self.ident, key, Self::FTL_ENUM_IDENT))
            .collect()
    }

    /// Returns the fields of the struct that are not skipped.
    pub fn fields(&self) -> Vec<&StructKvFieldOpts> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields
                .fields
                .iter()
                .filter(|field| !field.is_skipped())
                .collect(),
            _ => vec![],
        }
    }
}

/// Attribute arguments for a struct.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructKvFluentAttributeArgs {
    display: Option<String>,
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    #[darling(default)]
    this: Option<bool>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
}
impl StructKvFluentAttributeArgs {
    /// Returns the display strategy for the struct.
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
            DisplayStrategy::StdDisplay
        }
    }
    /// Returns `true` if the struct should be passed as `this`.
    pub fn is_this(&self) -> bool {
        self.this.unwrap_or(false)
    }
}
