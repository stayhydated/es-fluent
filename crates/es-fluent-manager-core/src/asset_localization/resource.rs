use es_fluent_shared::namespace::validate_namespace_path;
use std::collections::HashSet;
use std::fmt;
use unic_langid::LanguageIdentifier;

/// Stable key for a localized resource.
///
/// Keys use the canonical shape:
/// - `{domain}` for base files
/// - `{domain}/{namespace}` for namespaced files
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Creates a new resource key.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the domain segment of the key.
    pub fn domain(&self) -> &str {
        self.0.split('/').next().unwrap_or(self.as_str())
    }
}

impl AsRef<str> for ResourceKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for ResourceKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ResourceKey {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical description of a single localized resource file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleResourceSpec {
    /// Stable resource key used by managers (e.g., `my-crate`, `my-crate/ui`, `my-crate/ui/button`).
    pub key: ResourceKey,
    /// Path under a locale root (e.g., `my-crate.ftl`, `my-crate/ui.ftl`, `my-crate/ui/button.ftl`).
    pub locale_relative_path: String,
    /// Whether this resource is required for locale readiness.
    pub required: bool,
}

impl ModuleResourceSpec {
    /// Returns the full path for a locale (e.g., `en/my-crate.ftl`).
    pub fn locale_path(&self, lang: &LanguageIdentifier) -> String {
        format!("{}/{}", lang, self.locale_relative_path)
    }
}

fn module_resource_spec(
    key: impl Into<ResourceKey>,
    locale_relative_path: impl Into<String>,
    required: bool,
) -> ModuleResourceSpec {
    ModuleResourceSpec {
        key: key.into(),
        locale_relative_path: locale_relative_path.into(),
        required,
    }
}

/// Builds a canonical resource plan for a domain.
///
/// Contract:
/// - Without namespaces, `{domain}.ftl` is required.
/// - With namespaces, only `{domain}/{namespace}.ftl` entries are required.
pub fn resource_plan_for(domain: &str, namespaces: &[&str]) -> Vec<ModuleResourceSpec> {
    if namespaces.is_empty() {
        return vec![module_resource_spec(
            ResourceKey::new(domain.to_string()),
            format!("{domain}.ftl"),
            true,
        )];
    }

    let mut plan = Vec::with_capacity(namespaces.len());

    let mut seen = HashSet::new();
    for namespace in namespaces {
        debug_assert!(
            validate_namespace_path(namespace).is_ok(),
            "resource_plan_for received invalid namespace '{}'",
            namespace
        );

        let namespace = namespace.trim();
        if !seen.insert(namespace) {
            continue;
        }

        plan.push(module_resource_spec(
            ResourceKey::new(format!("{domain}/{namespace}")),
            format!("{domain}/{namespace}.ftl"),
            true,
        ));
    }

    plan
}

/// Returns required resource keys from a resource plan.
pub fn required_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<ResourceKey> {
    plan.iter()
        .filter(|spec| spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns optional resource keys from a resource plan.
pub fn optional_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<ResourceKey> {
    plan.iter()
        .filter(|spec| !spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns true when all required keys are present in the loaded set.
pub fn locale_is_ready(
    required_keys: &HashSet<ResourceKey>,
    loaded_keys: &HashSet<ResourceKey>,
) -> bool {
    required_keys.iter().all(|key| loaded_keys.contains(key))
}
