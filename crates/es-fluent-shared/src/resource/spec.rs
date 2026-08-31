use crate::LanguageIdentifier;
use crate::fluent::FluentDomain;
use crate::namespace::ResolvedNamespace;
use crate::registry::StaticFluentDomain;

use super::{LocaleRelativeFtlPath, ResourceKey, ResourcePlanError};

/// Canonical description of a single localized resource file.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct ModuleResourceSpec {
    /// Stable resource key used by managers (e.g., `my-crate`, `my-crate/ui`, `my-crate/ui/button`).
    pub key: ResourceKey,
    /// Path under a locale root (e.g., `my-crate.ftl`, `my-crate/ui.ftl`, `my-crate/ui/button.ftl`).
    pub locale_relative_path: LocaleRelativeFtlPath,
    /// Whether this resource is required for locale readiness.
    pub required: bool,
}

impl ModuleResourceSpec {
    /// Validates and creates a resource specification.
    pub fn try_new(
        key: impl Into<String>,
        locale_relative_path: impl Into<String>,
        required: bool,
    ) -> Result<Self, ResourcePlanError> {
        let key = key.into();
        let locale_relative_path = locale_relative_path.into();
        Ok(Self {
            key: ResourceKey::try_new(key.clone())
                .map_err(|details| ResourcePlanError::InvalidResourceKey { key, details })?,
            locale_relative_path: LocaleRelativeFtlPath::try_new(locale_relative_path.clone())
                .map_err(|details| ResourcePlanError::InvalidResourcePath {
                    path: locale_relative_path,
                    details,
                })?,
            required,
        })
    }

    /// Creates a resource specification from validated parts.
    pub fn new(
        key: ResourceKey,
        locale_relative_path: LocaleRelativeFtlPath,
        required: bool,
    ) -> Self {
        Self {
            key,
            locale_relative_path,
            required,
        }
    }

    /// Validates and creates the base domain resource specification.
    pub fn try_base(domain: &str, required: bool) -> Result<Self, ResourcePlanError> {
        Self::try_new(domain, format!("{domain}.ftl"), required)
    }

    /// Creates the base domain resource specification.
    pub fn base(domain: &str, required: bool) -> Self {
        let domain =
            FluentDomain::try_new(domain).expect("base resource domain should be validated");
        Self::base_for_domain(&domain, required)
    }

    /// Creates the base domain resource specification from a validated static domain.
    pub fn base_for_static_domain(domain: StaticFluentDomain, required: bool) -> Self {
        Self::new(
            ResourceKey::from_static_domain(domain),
            LocaleRelativeFtlPath::try_new(format!("{}.ftl", domain.as_str()))
                .expect("static domain should produce a valid locale-relative FTL path"),
            required,
        )
    }

    pub(super) fn base_for_domain(domain: &FluentDomain, required: bool) -> Self {
        Self::new(
            ResourceKey::from_domain(domain),
            LocaleRelativeFtlPath::try_new(format!("{}.ftl", domain.as_str()))
                .expect("domain should produce a valid locale-relative FTL path"),
            required,
        )
    }

    /// Validates and creates a namespaced resource specification.
    pub fn try_namespaced(
        domain: &str,
        namespace: &ResolvedNamespace,
        required: bool,
    ) -> Result<Self, ResourcePlanError> {
        Self::try_new(
            format!("{domain}/{namespace}"),
            format!("{domain}/{namespace}.ftl"),
            required,
        )
    }

    /// Creates a namespaced resource specification.
    pub fn namespaced(domain: &str, namespace: &ResolvedNamespace, required: bool) -> Self {
        let domain =
            FluentDomain::try_new(domain).expect("namespaced resource domain should be validated");
        Self::namespaced_for_domain(&domain, namespace, required)
    }

    /// Creates a namespaced resource specification from validated static metadata.
    pub fn namespaced_for_static_domain(
        domain: StaticFluentDomain,
        namespace: &ResolvedNamespace,
        required: bool,
    ) -> Self {
        Self::new(
            ResourceKey::from_static_domain_and_namespace(domain, namespace),
            LocaleRelativeFtlPath::try_new(format!("{}/{namespace}.ftl", domain.as_str())).expect(
                "static domain and namespace should produce a valid locale-relative FTL path",
            ),
            required,
        )
    }

    pub(super) fn namespaced_for_domain(
        domain: &FluentDomain,
        namespace: &ResolvedNamespace,
        required: bool,
    ) -> Self {
        Self::new(
            ResourceKey::from_domain_and_namespace(domain, namespace),
            LocaleRelativeFtlPath::try_new(format!("{}/{namespace}.ftl", domain.as_str()))
                .expect("domain and namespace should produce a valid locale-relative FTL path"),
            required,
        )
    }

    /// Returns the full path for a locale (e.g., `en/my-crate.ftl`).
    pub fn locale_path(&self, lang: &LanguageIdentifier) -> String {
        format!("{}/{}", lang, self.locale_relative_path)
    }
}
