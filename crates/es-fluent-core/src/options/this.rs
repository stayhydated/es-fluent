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
    /// If true, generates `ThisFtl` for the type itself.
    #[darling(default)]
    this: Option<bool>,
    /// If true, generates `ThisFtl` for the fields (replacing `keys_this`).
    #[darling(default)]
    fields_this: Option<bool>,
    /// If true, generates `ThisFtl` for the enum variants.
    #[darling(default)]
    variants_this: Option<bool>,
}

impl ThisAttributeArgs {
    /// Returns `true` if `this` should be generated.
    pub fn is_this(&self) -> bool {
        self.this.unwrap_or(true)
    }

    /// Returns `true` if `fields_this` should be generated.
    pub fn is_fields_this(&self) -> bool {
        self.fields_this.unwrap_or(false)
    }

    /// Returns `true` if `variants_this` should be generated.
    pub fn is_variants_this(&self) -> bool {
        self.variants_this.unwrap_or(false)
    }
}
