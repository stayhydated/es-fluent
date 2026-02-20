//! Shared module metadata and discovery contracts.

use fluent_bundle::FluentResource;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// Static metadata describing an i18n module.
///
/// This single shape is shared by all managers (embedded, Bevy, and future
/// third-party backends) so module discovery and routing can be standardized.
#[derive(Debug)]
pub struct ModuleData {
    /// The unique module name (typically crate name).
    pub name: &'static str,
    /// The Fluent domain for this module.
    pub domain: &'static str,
    /// Languages that this module can provide.
    pub supported_languages: &'static [LanguageIdentifier],
    /// Namespaces used by the module (e.g., "ui", "errors").
    /// If empty, only the main `{domain}.ftl` file is used.
    /// If non-empty, namespace files are the canonical resources and managers
    /// treat `{domain}.ftl` as optional compatibility data.
    pub namespaces: &'static [&'static str],
}

impl ModuleData {
    /// Returns the canonical resource plan for this module.
    pub fn resource_plan(&self) -> Vec<ModuleResourceSpec> {
        resource_plan_for(self.domain, self.namespaces)
    }
}

/// Canonical description of a single localized resource file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleResourceSpec {
    /// Stable resource key used by managers (e.g., `my-crate`, `my-crate/ui`).
    pub key: String,
    /// Path under a locale root (e.g., `my-crate.ftl`, `my-crate/ui.ftl`).
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
    key: impl Into<String>,
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
/// - With namespaces, `{domain}.ftl` is optional compatibility data and
///   `{domain}/{namespace}.ftl` entries are required.
pub fn resource_plan_for(domain: &str, namespaces: &[&str]) -> Vec<ModuleResourceSpec> {
    if namespaces.is_empty() {
        return vec![module_resource_spec(
            domain.to_string(),
            format!("{domain}.ftl"),
            true,
        )];
    }

    let mut plan = Vec::with_capacity(namespaces.len() + 1);
    plan.push(module_resource_spec(
        domain.to_string(),
        format!("{domain}.ftl"),
        false,
    ));

    let mut seen = HashSet::new();
    for namespace in namespaces {
        if !seen.insert(*namespace) {
            continue;
        }

        plan.push(module_resource_spec(
            format!("{domain}/{namespace}"),
            format!("{domain}/{namespace}.ftl"),
            true,
        ));
    }

    plan
}

/// Returns required resource keys from a resource plan.
pub fn required_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<String> {
    plan.iter()
        .filter(|spec| spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns optional resource keys from a resource plan.
pub fn optional_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<String> {
    plan.iter()
        .filter(|spec| !spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns true when all required keys are present in the loaded set.
pub fn locale_is_ready(required_keys: &HashSet<String>, loaded_keys: &HashSet<String>) -> bool {
    required_keys.iter().all(|key| loaded_keys.contains(key))
}

/// Canonical resource-load failure categories shared across managers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceLoadError {
    /// A required resource was not present.
    Missing {
        key: String,
        path: String,
        required: bool,
    },
    /// Resource bytes were not valid UTF-8.
    InvalidUtf8 {
        key: String,
        path: String,
        required: bool,
        details: String,
    },
    /// Resource content failed Fluent parsing.
    Parse {
        key: String,
        path: String,
        required: bool,
        details: String,
    },
}

impl ResourceLoadError {
    /// Constructs a missing-file error for a resource spec.
    pub fn missing(spec: &ModuleResourceSpec) -> Self {
        Self::Missing {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
        }
    }

    /// Returns true when this failure affects required readiness.
    pub fn is_required(&self) -> bool {
        match self {
            Self::Missing { required, .. }
            | Self::InvalidUtf8 { required, .. }
            | Self::Parse { required, .. } => *required,
        }
    }
}

impl fmt::Display for ResourceLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing {
                key,
                path,
                required,
            } => write!(
                f,
                "missing {} resource '{}' at '{}'",
                if *required { "required" } else { "optional" },
                key,
                path
            ),
            Self::InvalidUtf8 {
                key,
                path,
                required,
                details,
            } => write!(
                f,
                "invalid UTF-8 in {} resource '{}' at '{}': {}",
                if *required { "required" } else { "optional" },
                key,
                path,
                details
            ),
            Self::Parse {
                key,
                path,
                required,
                details,
            } => write!(
                f,
                "failed to parse {} resource '{}' at '{}': {}",
                if *required { "required" } else { "optional" },
                key,
                path,
                details
            ),
        }
    }
}

impl std::error::Error for ResourceLoadError {}

/// Parses UTF-8 bytes into a `FluentResource` using the shared load contract.
pub fn parse_fluent_resource_bytes(
    spec: &ModuleResourceSpec,
    bytes: &[u8],
) -> Result<Arc<FluentResource>, ResourceLoadError> {
    let content =
        String::from_utf8(bytes.to_vec()).map_err(|e| ResourceLoadError::InvalidUtf8 {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
            details: e.to_string(),
        })?;

    parse_fluent_resource_content(spec, content)
}

/// Parses Fluent source text into a `FluentResource` using the shared load contract.
pub fn parse_fluent_resource_content(
    spec: &ModuleResourceSpec,
    content: String,
) -> Result<Arc<FluentResource>, ResourceLoadError> {
    FluentResource::try_new(content)
        .map(Arc::new)
        .map_err(|(_, errs)| ResourceLoadError::Parse {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
            details: format!("{errs:?}"),
        })
}

/// Common discovery contract for managers.
///
/// Any backend can iterate this inventory to discover registered modules.
pub trait I18nModuleDescriptor: Send + Sync {
    /// Returns static metadata for this module.
    fn data(&self) -> &'static ModuleData;
}

/// A simple descriptor wrapper for metadata-only registrations.
///
/// This is used by asset-driven managers (e.g., Bevy) where runtime localization
/// is handled by the host runtime rather than by `Localizer`.
pub struct StaticModuleDescriptor {
    data: &'static ModuleData,
}

impl StaticModuleDescriptor {
    /// Creates a new metadata-only descriptor.
    pub const fn new(data: &'static ModuleData) -> Self {
        Self { data }
    }
}

impl I18nModuleDescriptor for StaticModuleDescriptor {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

inventory::collect!(&'static dyn I18nModuleDescriptor);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use unic_langid::langid;

    static SUPPORTED: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
    static NAMESPACES: &[&str] = &["ui", "errors"];
    static DATA: ModuleData = ModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED,
        namespaces: NAMESPACES,
    };

    #[test]
    fn static_descriptor_new_and_data_round_trip() {
        let module = StaticModuleDescriptor::new(&DATA);
        let data = module.data();

        assert_eq!(data.name, "test-module");
        assert_eq!(data.domain, "test-domain");
        assert_eq!(data.supported_languages, SUPPORTED);
        assert_eq!(data.namespaces, NAMESPACES);
    }

    #[test]
    fn resource_plan_without_namespaces_requires_base_file() {
        let plan = resource_plan_for("app", &[]);
        assert_eq!(
            plan,
            vec![ModuleResourceSpec {
                key: "app".to_string(),
                locale_relative_path: "app.ftl".to_string(),
                required: true
            }]
        );
    }

    #[test]
    fn resource_plan_with_namespaces_requires_namespace_files() {
        let plan = resource_plan_for("app", &["ui", "errors"]);
        assert_eq!(
            plan,
            vec![
                ModuleResourceSpec {
                    key: "app".to_string(),
                    locale_relative_path: "app.ftl".to_string(),
                    required: false
                },
                ModuleResourceSpec {
                    key: "app/ui".to_string(),
                    locale_relative_path: "app/ui.ftl".to_string(),
                    required: true
                },
                ModuleResourceSpec {
                    key: "app/errors".to_string(),
                    locale_relative_path: "app/errors.ftl".to_string(),
                    required: true
                }
            ]
        );
        assert_eq!(plan[1].locale_path(&langid!("en-US")), "en-US/app/ui.ftl");
    }

    #[test]
    fn resource_plan_deduplicates_duplicate_namespaces() {
        let plan = resource_plan_for("app", &["ui", "ui"]);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[1].key, "app/ui");
    }

    #[test]
    fn locale_is_ready_requires_all_required_keys() {
        let plan = resource_plan_for("app", &["ui", "errors"]);
        let required = required_resource_keys_from_plan(&plan);
        let optional = optional_resource_keys_from_plan(&plan);

        assert_eq!(optional, HashSet::from(["app".to_string()]));

        let ready_loaded = HashSet::from(["app/ui".to_string(), "app/errors".to_string()]);
        assert!(locale_is_ready(&required, &ready_loaded));

        let missing_required = HashSet::from(["app/ui".to_string()]);
        assert!(!locale_is_ready(&required, &missing_required));
    }

    #[test]
    fn module_data_resource_plan_delegates_to_shared_builder() {
        let plan = DATA.resource_plan();
        let direct = resource_plan_for(DATA.domain, DATA.namespaces);
        assert_eq!(plan, direct);
    }

    #[test]
    fn parse_fluent_resource_content_reports_parse_errors() {
        let spec = ModuleResourceSpec {
            key: "app/ui".to_string(),
            locale_relative_path: "app/ui.ftl".to_string(),
            required: true,
        };

        let err = parse_fluent_resource_content(&spec, "broken = {".to_string())
            .expect_err("invalid fluent should fail");
        assert!(matches!(
            err,
            ResourceLoadError::Parse { required: true, .. }
        ));
    }

    #[test]
    fn parse_fluent_resource_bytes_reports_utf8_errors() {
        let spec = ModuleResourceSpec {
            key: "app/ui".to_string(),
            locale_relative_path: "app/ui.ftl".to_string(),
            required: false,
        };

        let err =
            parse_fluent_resource_bytes(&spec, &[0xFF, 0xFE]).expect_err("invalid utf-8 bytes");
        assert!(matches!(
            err,
            ResourceLoadError::InvalidUtf8 {
                required: false,
                ..
            }
        ));
    }
}
