use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta, FromVariant};
use getset::Getters;
use heck::ToSnakeCase as _;
use quote::format_ident;

use crate::error::EsFluentCoreResult;

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
    /// A value transformation expression.
    #[darling(default)]
    value: Option<super::ValueAttr>,
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
    /// Returns the value expression if present.
    pub fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref().map(|v| &v.0)
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
    /// Overrides the localization key suffix for this variant.
    #[darling(default)]
    key: Option<String>,
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
    /// Returns the explicit localization key for the variant, if provided.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

/// Options for an enum.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
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

    /// Returns the base localization key used for this enum.
    pub fn base_key(&self) -> String {
        if let Some(resource) = self.attr_args().resource() {
            resource.to_string()
        } else {
            self.ident().to_string().to_snake_case()
        }
    }
}

/// Attribute arguments for an enum.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct EnumFluentAttributeArgs {
    #[darling(default)]
    resource: Option<String>,
    /// Whether to skip inventory registration for this enum.
    /// Used by `#[es_fluent_language]` to prevent language enums from being registered.
    #[darling(default)]
    skip_inventory: Option<bool>,
}

impl EnumFluentAttributeArgs {
    /// Returns the explicit resource base key if provided.
    pub fn resource(&self) -> Option<&str> {
        self.resource.as_deref()
    }

    /// Returns `true` if inventory registration should be skipped.
    pub fn skip_inventory(&self) -> bool {
        self.skip_inventory.unwrap_or(false)
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

/// Options for an enum variant in EsFluentVariants context.
#[derive(Clone, Debug, FromVariant, Getters)]
#[darling(attributes(fluent_kv))]
pub struct EnumKvVariantOpts {
    /// The identifier of the variant.
    #[getset(get = "pub")]
    ident: syn::Ident,
    fields: darling::ast::Fields<darling::util::Ignored>,
    /// Whether to skip this variant.
    #[darling(default)]
    skip: Option<bool>,
}

impl EnumKvVariantOpts {
    /// Returns `true` if the variant should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    /// Returns the style of the variant's fields.
    pub fn style(&self) -> darling::ast::Style {
        self.fields.style
    }

    /// Returns true if this is a tuple variant with exactly one field.
    pub fn is_single_tuple(&self) -> bool {
        matches!(self.fields.style, darling::ast::Style::Tuple) && self.fields.len() == 1
    }
}

/// Options for an enum with EsFluentVariants.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(enum_unit, enum_named, enum_tuple), attributes(fluent_kv))]
#[getset(get = "pub")]
pub struct EnumKvOpts {
    /// The identifier of the enum.
    ident: syn::Ident,
    /// The generics of the enum.
    generics: syn::Generics,
    data: darling::ast::Data<EnumKvVariantOpts, darling::util::Ignored>,
    #[darling(flatten)]
    attr_args: EnumKvFluentAttributeArgs,
}

impl EnumKvOpts {
    const FTL_ENUM_IDENT: &str = "Variants";

    /// Returns the identifier of the FTL enum.
    pub fn ftl_enum_ident(&self) -> syn::Ident {
        format_ident!("{}{}", &self.ident, Self::FTL_ENUM_IDENT)
    }

    /// Returns the identifiers of the keyed FTL enums.
    pub fn keyyed_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        self.attr_args.clone().keys.map_or_else(
            || Ok(Vec::new()),
            |keys| {
                keys.into_iter()
                    .map(|key| {
                        let pascal_key = super::validate_snake_case_key(&key)?;
                        Ok(format_ident!(
                            "{}{}{}",
                            &self.ident,
                            pascal_key,
                            Self::FTL_ENUM_IDENT
                        ))
                    })
                    .collect()
            },
        )
    }

    /// Returns the identifiers used to build base FTL keys (without suffixes).
    pub fn keyed_base_idents(&self) -> EsFluentCoreResult<Vec<syn::Ident>> {
        self.attr_args.clone().keys.map_or_else(
            || Ok(Vec::new()),
            |keys| {
                keys.into_iter()
                    .map(|key| {
                        let pascal_key = super::validate_snake_case_key(&key)?;
                        Ok(format_ident!("{}{}", &self.ident, pascal_key))
                    })
                    .collect()
            },
        )
    }

    /// Returns the variants of the enum that are not skipped.
    pub fn variants(&self) -> Vec<&EnumKvVariantOpts> {
        match &self.data {
            darling::ast::Data::Enum(variants) => {
                variants.iter().filter(|v| !v.is_skipped()).collect()
            },
            _ => unreachable!("Unexpected data type for enum"),
        }
    }
}

/// Attribute arguments for an enum with EsFluentVariants.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct EnumKvFluentAttributeArgs {
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
}
