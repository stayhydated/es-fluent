use bon::Builder;
use darling::{FromDeriveInput, FromMeta};
use getset::Getters;

/// Options for `EsFluentThis`.
#[derive(Clone, Debug, FromDeriveInput, Getters)]
#[darling(supports(struct_any, enum_any), attributes(fluent_this))]
#[getset(get = "pub")]
pub struct ThisOpts {
    /// The identifier of the struct/enum.
    ident: syn::Ident,
    /// The generics of the struct/enum.
    generics: syn::Generics,
    #[darling(flatten)]
    attr_args: ThisAttributeArgs,
}

/// Attribute arguments for `EsFluentThis`.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct ThisAttributeArgs {
    #[darling(default)]
    origin: Option<bool>,
    #[darling(default)]
    fields: Option<bool>,
    #[darling(default)]
    variants: Option<bool>,
}

impl ThisAttributeArgs {
    /// Returns `true` if `origin` should be generated.
    pub fn is_origin(&self) -> bool {
        self.origin.unwrap_or(true)
    }

    /// Returns `true` if `fields` should be generated.
    pub fn is_fields(&self) -> bool {
        self.fields.unwrap_or(false)
    }

    /// Returns `true` if `variants` should be generated.
    pub fn is_variants(&self) -> bool {
        self.variants.unwrap_or(false)
    }
}
