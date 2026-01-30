use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta};
use getset::Getters;
use quote::format_ident;

use crate::error::EsFluentCoreResult;
use crate::namer;

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
    /// A value transformation expression.
    #[darling(default)]
    value: Option<super::ValueAttr>,
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

    /// Returns the Fluent argument name for this field.
    pub fn fluent_arg_name(&self, index: usize) -> String {
        self.ident
            .as_ref()
            .map(|ident| ident.to_string())
            .unwrap_or_else(|| namer::UnnamedItem::from(index).to_string())
    }

    /// Returns the value expression if present.
    pub fn value(&self) -> Option<&syn::Expr> {
        self.value.as_ref().map(|v| &v.0)
    }
}

/// Options for a struct.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_named, struct_tuple, struct_unit), attributes(fluent))]
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

    /// Returns the fields of the struct paired with their declaration index.
    pub fn indexed_fields(&self) -> Vec<(usize, &StructFieldOpts)> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| !field.is_skipped())
                .collect(),
            _ => vec![],
        }
    }

    /// Returns all fields (including skipped) paired with their declaration index.
    pub fn all_indexed_fields(&self) -> Vec<(usize, &StructFieldOpts)> {
        match &self.data {
            darling::ast::Data::Struct(fields) => fields.fields.iter().enumerate().collect(),
            _ => vec![],
        }
    }
}

/// Attribute arguments for a struct.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructFluentAttributeArgs {
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace(file(named))` - literal "file" namespace
    #[darling(default)]
    namespace: Option<super::namespace::NamespaceValue>,
}
impl StructFluentAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace.as_ref()
    }
}

/// Options for a struct field.
#[derive(Clone, Debug, FromField, Getters)]
#[darling(attributes(fluent_variants))]
pub struct StructVariantsFieldOpts {
    /// The identifier of the field.
    #[getset(get = "pub")]
    ident: Option<syn::Ident>,
    /// The type of the field.
    #[getset(get = "pub")]
    ty: syn::Type,
    /// Whether to skip this field.
    #[darling(default)]
    skip: Option<bool>,
}

impl StructVariantsFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
}

/// Options for a struct.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_named, struct_unit), attributes(fluent_variants))]
#[getset(get = "pub")]
pub struct StructVariantsOpts {
    /// The identifier of the struct.
    ident: syn::Ident,
    /// The generics of the struct.
    generics: syn::Generics,
    data: darling::ast::Data<darling::util::Ignored, StructVariantsFieldOpts>,
    #[darling(flatten)]
    attr_args: StructVariantsFluentAttributeArgs,
}

impl StructVariantsOpts {
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

    /// Returns the fields of the struct that are not skipped.
    pub fn fields(&self) -> Vec<&StructVariantsFieldOpts> {
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
pub struct StructVariantsFluentAttributeArgs {
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
}
