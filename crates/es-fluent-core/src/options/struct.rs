use bon::Builder;
use darling::{FromDeriveInput, FromField, FromMeta};
use getset::Getters;
use heck::{ToPascalCase as _, ToSnakeCase as _};
use quote::format_ident;

use crate::error::{ErrorExt as _, EsFluentCoreError, EsFluentCoreResult};
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
    value: Option<ValueAttr>,
}

#[derive(Clone, Debug)]
pub struct ValueAttr(pub syn::Expr);

impl darling::FromMeta for ValueAttr {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::List(list) => {
                let expr: syn::Expr = syn::parse2(list.tokens.clone())?;
                Ok(ValueAttr(expr))
            }
            syn::Meta::NameValue(nv) => {
                // Also support value = "expr" for convenience?
                 if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                    let expr: syn::Expr = s.parse()?;
                    Ok(ValueAttr(expr))
                } else {
                     Err(darling::Error::unexpected_type("non-string literal"))
                }
            }
             _ => Err(darling::Error::unsupported_format("list or name-value")),
        }
    }
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
}
impl StructFluentAttributeArgs {}

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
}

impl StructKvFieldOpts {
    /// Returns `true` if the field should be skipped.
    pub fn is_skipped(&self) -> bool {
        self.skip.unwrap_or(false)
    }
}

/// Options for a struct.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_named, struct_unit), attributes(fluent_kv))]
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
    const FTL_ENUM_IDENT: &str = "KvFtl";

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
                        let pascal_key = Self::validate_key(&key)?;
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

    fn validate_key(key: &syn::LitStr) -> EsFluentCoreResult<String> {
        let key_str = key.value();
        let snake_cased = key_str.to_snake_case();
        let is_lower_snake = !key_str.is_empty()
            && key_str == snake_cased
            && key_str == key_str.to_ascii_lowercase();

        if !is_lower_snake {
            return Err(EsFluentCoreError::AttributeError {
                message: format!(
                    "keys in #[fluent_kv] must be lowercase snake_case; found \"{}\"",
                    key_str
                ),
                span: Some(key.span()),
            }
            .with_help("Use values like \"description\" or \"label\".".to_string()));
        }

        Ok(key_str.to_pascal_case())
    }
}

/// Attribute arguments for a struct.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct StructKvFluentAttributeArgs {
    #[darling(default)]
    keys: Option<Vec<syn::LitStr>>,
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
}
impl StructKvFluentAttributeArgs {}
