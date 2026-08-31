use crate::namespace::SpannedNamespaceRule;
use crate::semantic::{DomainName, GeneratedKeyName, SpannedValue};
use bon::Builder;
use darling::FromMeta;
use es_fluent_shared::namespace::NamespaceRule;
use getset::Getters;

use super::GeneratedKeyList;

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct NamespacedAttributeArgs {
    /// Optional namespace for FTL file generation.
    /// - `namespace = "name"` - writes to `{lang}/{crate}/{name}.ftl`
    /// - `namespace = file` - writes to `{lang}/{crate}/{source_file_stem}.ftl`
    /// - `namespace = file_relative` - writes to `{lang}/{crate}/{relative_path}.ftl`
    /// - `namespace = folder` - writes to `{lang}/{crate}/{source_parent_folder}.ftl`
    /// - `namespace = folder_relative` - writes to `{lang}/{crate}/{relative_parent_folder_path}.ftl`
    #[darling(default)]
    namespace: Option<SpannedNamespaceRule>,
}

impl NamespacedAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace.as_ref().map(SpannedNamespaceRule::rule)
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace.as_ref().map(SpannedNamespaceRule::span)
    }

    /// Returns the parsed namespace spec if provided.
    pub fn namespace_spec(&self) -> Option<&SpannedNamespaceRule> {
        self.namespace.as_ref()
    }
}

/// Shared explicit domain and namespace arguments for message containers.
#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct ScopedAttributeArgs {
    #[darling(default)]
    domain: Option<SpannedValue<DomainName>>,
    #[darling(flatten)]
    namespace_args: NamespacedAttributeArgs,
}

impl ScopedAttributeArgs {
    /// Returns the typed explicit package-local domain.
    pub fn domain_name(&self) -> Option<&SpannedValue<DomainName>> {
        self.domain.as_ref()
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace_args.namespace_span()
    }

    /// Returns the parsed namespace spec if provided.
    pub fn namespace_spec(&self) -> Option<&SpannedNamespaceRule> {
        self.namespace_args.namespace_spec()
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct DerivedNamespacedAttributeArgs {
    /// The traits to derive on the FTL enum.
    #[getset(get = "pub")]
    #[darling(default)]
    derive: darling::util::PathList,
    #[darling(flatten)]
    namespace_args: NamespacedAttributeArgs,
}

impl DerivedNamespacedAttributeArgs {
    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.namespace_args.namespace_span()
    }
}

#[derive(Builder, Clone, Debug, Default, FromMeta, Getters)]
pub struct VariantsFluentAttributeArgs {
    #[darling(default)]
    pub(super) keys: Option<GeneratedKeyList>,
    #[darling(flatten)]
    derived_args: DerivedNamespacedAttributeArgs,
}

impl VariantsFluentAttributeArgs {
    /// Returns the traits to derive on the generated enum.
    pub fn derive(&self) -> &darling::util::PathList {
        self.derived_args.derive()
    }

    /// Returns the typed generated variant keys if provided.
    pub fn keys(&self) -> Option<&[SpannedValue<GeneratedKeyName>]> {
        self.keys.as_ref().map(GeneratedKeyList::as_slice)
    }

    /// Returns a span inside the explicit key list when provided.
    pub fn keys_span(&self) -> Option<proc_macro2::Span> {
        self.keys.as_ref().and_then(GeneratedKeyList::span)
    }

    /// Returns the namespace value if provided.
    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.derived_args.namespace()
    }

    /// Returns the span of the namespace value if provided.
    pub fn namespace_span(&self) -> Option<proc_macro2::Span> {
        self.derived_args.namespace_span()
    }
}
