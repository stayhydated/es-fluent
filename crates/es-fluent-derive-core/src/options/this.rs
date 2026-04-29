use bon::Builder;
use darling::{FromDeriveInput, FromMeta};
use es_fluent_shared::namespace::NamespaceRule;
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
    attr_args: ThisNamespacedAttributeArgs,
}

/// Attribute arguments for `EsFluentThis`.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct ThisNamespacedAttributeArgs {
    #[darling(default)]
    origin: Option<bool>,
    #[darling(default)]
    variants: Option<bool>,
    #[darling(flatten)]
    namespace_args: super::NamespacedAttributeArgs,
}

impl ThisNamespacedAttributeArgs {
    /// Returns `true` if `origin` should be generated.
    pub fn is_origin(&self) -> bool {
        self.origin.unwrap_or(true)
    }

    /// Returns `true` if `variants` should be generated.
    pub fn is_variants(&self) -> bool {
        self.variants.unwrap_or(false)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }
}
