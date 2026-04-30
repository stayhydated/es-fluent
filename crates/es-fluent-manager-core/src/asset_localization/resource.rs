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
/// - With namespaces, `{domain}.ftl` remains an optional mixed-mode resource
///   and `{domain}/{namespace}.ftl` entries are required.
/// - Invalid namespace paths panic before any resource paths are produced.
pub fn resource_plan_for(domain: &str, namespaces: &[&str]) -> Vec<ModuleResourceSpec> {
    if namespaces.is_empty() {
        return vec![module_resource_spec(
            ResourceKey::new(domain.to_string()),
            format!("{domain}.ftl"),
            true,
        )];
    }

    let mut plan = Vec::with_capacity(namespaces.len() + 1);
    plan.push(module_resource_spec(
        ResourceKey::new(domain.to_string()),
        format!("{domain}.ftl"),
        false,
    ));

    let mut seen = HashSet::new();
    for namespace in namespaces {
        if let Err(error) = es_fluent_shared::namespace::validate_namespace_path(namespace) {
            panic!("resource_plan_for received invalid namespace '{namespace}': {error}");
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    #[test]
    fn resource_key_conversions_preserve_key_and_domain() {
        let from_string: ResourceKey = "demo/ui".to_string().into();
        let from_str: ResourceKey = "demo/errors".into();

        assert_eq!(from_string.as_str(), "demo/ui");
        assert_eq!(from_string.domain(), "demo");
        assert_eq!(from_string.as_ref(), "demo/ui");
        assert_eq!(from_str.to_string(), "demo/errors");
    }

    #[test]
    fn resource_plan_for_handles_base_and_namespaced_resources() {
        let base_plan = resource_plan_for("demo", &[]);
        assert_eq!(base_plan.len(), 1);
        assert_eq!(base_plan[0].key.as_str(), "demo");
        assert_eq!(base_plan[0].locale_relative_path, "demo.ftl");
        assert_eq!(
            base_plan[0].locale_path(&langid!("en-US")),
            "en-US/demo.ftl"
        );
        assert!(base_plan[0].required);

        let namespaced_plan = resource_plan_for("demo", &["ui", "ui", "errors"]);
        let keys: Vec<_> = namespaced_plan
            .iter()
            .map(|spec| spec.key.as_str())
            .collect();
        assert_eq!(keys, vec!["demo", "demo/ui", "demo/errors"]);
        assert!(!namespaced_plan[0].required);
        assert_eq!(namespaced_plan[0].locale_relative_path, "demo.ftl");
        assert!(namespaced_plan[1].required);
        assert!(
            namespaced_plan
                .iter()
                .all(|spec| spec.locale_relative_path.ends_with(".ftl"))
        );
    }

    #[test]
    #[should_panic(expected = "resource_plan_for received invalid namespace")]
    fn resource_plan_for_rejects_invalid_namespaces() {
        let _ = resource_plan_for("demo", &["../outside"]);
    }

    #[test]
    fn required_and_optional_keys_reflect_plan_membership() {
        let plan = vec![
            module_resource_spec("demo", "demo.ftl", true),
            module_resource_spec("demo/optional", "demo/optional.ftl", false),
        ];

        let required = required_resource_keys_from_plan(&plan);
        let optional = optional_resource_keys_from_plan(&plan);

        assert!(required.contains(&ResourceKey::from("demo")));
        assert!(!required.contains(&ResourceKey::from("demo/optional")));
        assert!(optional.contains(&ResourceKey::from("demo/optional")));
        assert!(!optional.contains(&ResourceKey::from("demo")));

        let loaded = HashSet::from([ResourceKey::from("demo")]);
        assert!(locale_is_ready(&required, &loaded));

        let unloaded = HashSet::new();
        assert!(!locale_is_ready(&required, &unloaded));
    }
}
