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
    members: Option<bool>,
    /// Optional namespace for FTL file generation.
    #[darling(default)]
    namespace: Option<super::namespace::NamespaceValue>,
}

impl ThisAttributeArgs {
    /// Returns `true` if `origin` should be generated.
    pub fn is_origin(&self) -> bool {
        self.origin.unwrap_or(true)
    }

    /// Returns `true` if `members` should be generated.
    pub fn is_members(&self) -> bool {
        self.members.unwrap_or(false)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&super::namespace::NamespaceValue> {
        self.namespace.as_ref()
    }
}
