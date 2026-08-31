use crate::fluent::FluentDomain;
use crate::namespace::{NamespacePathError, ResolvedNamespace};
use crate::registry::StaticFluentDomain;

/// Error produced while validating a resource key.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
#[error("{0}")]
pub struct ResourceKeyError(#[from] pub(super) NamespacePathError);

/// Stable key for a localized resource.
///
/// Keys use the canonical shape:
/// - `{domain}` for base files
/// - `{domain}/{namespace}` for namespaced files
#[derive(
    Clone, Debug, derive_more::AsRef, derive_more::Display, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
#[as_ref(str)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Validates and creates a resource key.
    pub fn try_new(key: impl Into<String>) -> Result<Self, ResourceKeyError> {
        let key = key.into();
        crate::namespace::validate_namespace_path_typed(&key)?;
        Ok(Self(key))
    }

    /// Validates and creates a resource key from static metadata.
    #[allow(
        clippy::panic,
        reason = "static metadata uses literal keys; use try_new for dynamic input"
    )]
    pub fn from_static_path(key: &'static str) -> Self {
        Self::try_new(key)
            .unwrap_or_else(|error| panic!("invalid static resource key '{key}': {error}"))
    }

    /// Creates a base resource key from an already-validated static Fluent domain.
    pub fn from_static_domain(domain: StaticFluentDomain) -> Self {
        Self(domain.as_str().to_string())
    }

    /// Creates a namespaced resource key from already-validated domain and namespace values.
    pub fn from_static_domain_and_namespace(
        domain: StaticFluentDomain,
        namespace: &ResolvedNamespace,
    ) -> Self {
        Self(format!("{}/{namespace}", domain.as_str()))
    }

    pub(super) fn from_domain(domain: &FluentDomain) -> Self {
        Self(domain.as_str().to_string())
    }

    pub(super) fn from_domain_and_namespace(
        domain: &FluentDomain,
        namespace: &ResolvedNamespace,
    ) -> Self {
        Self(format!("{}/{namespace}", domain.as_str()))
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the domain segment of the key.
    pub fn domain(&self) -> &str {
        self.0.split('/').next().unwrap_or(self.as_str())
    }

    /// Returns the domain segment as a validated Fluent domain.
    pub fn domain_name(&self) -> FluentDomain {
        FluentDomain::try_new(self.domain())
            .expect("resource key validation should guarantee a valid domain segment")
    }
}

impl serde::Serialize for ResourceKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ResourceKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}
