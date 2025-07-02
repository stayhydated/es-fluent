use std::str::FromStr as _;

use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta};
use getset::Getters;
use quote::format_ident;
use strum::IntoEnumIterator as _;

use crate::strategy::DisplayStrategy;

#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent))]
pub struct StructFieldOpts {
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    #[getset(get = "pub")]
    ty: syn::Type,
    #[darling(default)]
    skip: Option<bool>,
    #[darling(default)]
    default: Option<bool>,
}

impl StructFieldOpts {
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    pub fn is_default(&self) -> bool {
        self.default.unwrap_or(false)
    }
}

#[derive(Clone, FromDeriveInput, Getters)]
#[darling(supports(struct_named), attributes(fluent))]
#[getset(get = "pub")]
pub struct StructOpts {
    ident: syn::Ident,
    #[darling(default)]
    derive: darling::util::PathList,
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, StructFieldOpts>,
    #[darling(flatten)]
    attr_args: StructFluentAttributeArgs,
}

impl StructOpts {
    const FTL_ENUM_IDENT: &str = "Ftl";

    pub fn ftl_enum_ident(&self) -> syn::Ident {
        format_ident!("{}{}", &self.ident, Self::FTL_ENUM_IDENT)
    }

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

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructFluentAttributeArgs {
    #[darling(default)]
    fl: Option<syn::Path>, // this is probably useless, force user to provide a fl! macro?
    display: Option<String>,
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
}
impl StructFluentAttributeArgs {
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
            DisplayStrategy::StdDisplay
        }
    }
}
