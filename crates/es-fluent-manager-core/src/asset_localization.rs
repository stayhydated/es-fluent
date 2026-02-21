//! Shared module metadata and discovery contracts.

use fluent_bundle::FluentResource;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;
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

/// Validation failures for a discovered module registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleRegistryError {
    /// A module has an empty name.
    EmptyModuleName,
    /// A module has an empty domain.
    EmptyDomain { module: String },
    /// A module name appears more than once.
    DuplicateModuleName { name: String },
    /// A domain appears more than once.
    DuplicateDomain { domain: String },
    /// A module declares the same language more than once.
    DuplicateSupportedLanguage {
        module: String,
        language: LanguageIdentifier,
    },
    /// A module declares the same namespace more than once.
    DuplicateNamespace { module: String, namespace: String },
    /// A namespace entry is malformed.
    InvalidNamespace {
        module: String,
        namespace: String,
        details: &'static str,
    },
}

impl fmt::Display for ModuleRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyModuleName => write!(f, "module name must not be empty"),
            Self::EmptyDomain { module } => {
                write!(f, "module '{}' has an empty domain", module)
            },
            Self::DuplicateModuleName { name } => {
                write!(f, "duplicate module name '{}'", name)
            },
            Self::DuplicateDomain { domain } => {
                write!(f, "duplicate module domain '{}'", domain)
            },
            Self::DuplicateSupportedLanguage { module, language } => write!(
                f,
                "module '{}' declares duplicate language '{}'",
                module, language
            ),
            Self::DuplicateNamespace { module, namespace } => write!(
                f,
                "module '{}' declares duplicate namespace '{}'",
                module, namespace
            ),
            Self::InvalidNamespace {
                module,
                namespace,
                details,
            } => write!(
                f,
                "module '{}' has invalid namespace '{}': {}",
                module, namespace, details
            ),
        }
    }
}

impl std::error::Error for ModuleRegistryError {}

/// Validates module metadata discovered through inventory.
///
/// Contract:
/// - `name` and `domain` must be non-empty.
/// - `name` and `domain` must be globally unique.
/// - `supported_languages` and `namespaces` must not contain duplicates.
/// - Namespaces must be bare namespace names (no slash and no `.ftl` suffix).
pub fn validate_module_registry<'a>(
    modules: impl IntoIterator<Item = &'a ModuleData>,
) -> Result<(), Vec<ModuleRegistryError>> {
    let mut errors = Vec::new();
    let mut module_names = HashSet::new();
    let mut module_domains = HashSet::new();

    for data in modules {
        if data.name.trim().is_empty() {
            errors.push(ModuleRegistryError::EmptyModuleName);
        } else if !module_names.insert(data.name) {
            errors.push(ModuleRegistryError::DuplicateModuleName {
                name: data.name.to_string(),
            });
        }

        if data.domain.trim().is_empty() {
            errors.push(ModuleRegistryError::EmptyDomain {
                module: data.name.to_string(),
            });
        } else if !module_domains.insert(data.domain) {
            errors.push(ModuleRegistryError::DuplicateDomain {
                domain: data.domain.to_string(),
            });
        }

        let mut seen_languages = HashSet::new();
        for lang in data.supported_languages {
            if !seen_languages.insert(lang.clone()) {
                errors.push(ModuleRegistryError::DuplicateSupportedLanguage {
                    module: data.name.to_string(),
                    language: lang.clone(),
                });
            }
        }

        let mut seen_namespaces = HashSet::new();
        for namespace in data.namespaces {
            let trimmed = namespace.trim();
            if trimmed.is_empty() {
                errors.push(ModuleRegistryError::InvalidNamespace {
                    module: data.name.to_string(),
                    namespace: namespace.to_string(),
                    details: "namespace must not be empty",
                });
                continue;
            }
            if trimmed.contains('/') {
                errors.push(ModuleRegistryError::InvalidNamespace {
                    module: data.name.to_string(),
                    namespace: namespace.to_string(),
                    details: "namespace must not contain '/'",
                });
            }
            if trimmed.ends_with(".ftl") {
                errors.push(ModuleRegistryError::InvalidNamespace {
                    module: data.name.to_string(),
                    namespace: namespace.to_string(),
                    details: "namespace must not include file extension",
                });
            }
            if !seen_namespaces.insert(trimmed) {
                errors.push(ModuleRegistryError::DuplicateNamespace {
                    module: data.name.to_string(),
                    namespace: trimmed.to_string(),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Canonical description of a single localized resource file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleResourceSpec {
    /// Stable resource key used by managers (e.g., `my-crate`, `my-crate/ui`).
    pub key: ResourceKey,
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
/// - With namespaces, `{domain}.ftl` is optional compatibility data and
///   `{domain}/{namespace}.ftl` entries are required.
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
        if !seen.insert(*namespace) {
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

/// Structured locale loading state shared across managers.
#[derive(Clone, Debug, Default)]
pub struct LocaleLoadReport {
    required_keys: HashSet<ResourceKey>,
    optional_keys: HashSet<ResourceKey>,
    loaded_keys: HashSet<ResourceKey>,
    errors: Vec<ResourceLoadError>,
}

impl LocaleLoadReport {
    /// Builds a new report from a canonical resource plan.
    pub fn from_plan(plan: &[ModuleResourceSpec]) -> Self {
        Self::from_specs(plan.iter())
    }

    /// Builds a new report from resource specs.
    pub fn from_specs<'a>(specs: impl IntoIterator<Item = &'a ModuleResourceSpec>) -> Self {
        let mut required_keys = HashSet::new();
        let mut optional_keys = HashSet::new();

        for spec in specs {
            if spec.required {
                required_keys.insert(spec.key.clone());
            } else {
                optional_keys.insert(spec.key.clone());
            }
        }

        Self {
            required_keys,
            optional_keys,
            loaded_keys: HashSet::new(),
            errors: Vec::new(),
        }
    }

    /// Marks a resource key as loaded.
    pub fn mark_loaded(&mut self, key: ResourceKey) {
        self.loaded_keys.insert(key);
    }

    /// Records a resource load error and removes the corresponding loaded key.
    pub fn record_error(&mut self, error: ResourceLoadError) {
        self.loaded_keys.remove(error.key());
        self.errors.push(error);
    }

    /// Returns required keys from the report.
    pub fn required_keys(&self) -> &HashSet<ResourceKey> {
        &self.required_keys
    }

    /// Returns optional keys from the report.
    pub fn optional_keys(&self) -> &HashSet<ResourceKey> {
        &self.optional_keys
    }

    /// Returns loaded keys from the report.
    pub fn loaded_keys(&self) -> &HashSet<ResourceKey> {
        &self.loaded_keys
    }

    /// Returns all recorded load errors.
    pub fn errors(&self) -> &[ResourceLoadError] {
        &self.errors
    }

    /// Returns required keys that are still missing.
    pub fn missing_required_keys(&self) -> HashSet<ResourceKey> {
        self.required_keys
            .iter()
            .filter(|key| !self.loaded_keys.contains(*key))
            .cloned()
            .collect()
    }

    /// Returns true when a required resource failed loading.
    pub fn has_required_errors(&self) -> bool {
        self.errors.iter().any(ResourceLoadError::is_required)
    }

    /// Returns true when locale readiness requirements are met.
    pub fn is_ready(&self) -> bool {
        locale_is_ready(&self.required_keys, &self.loaded_keys) && !self.has_required_errors()
    }
}

/// Canonical resource-load failure categories shared across managers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceLoadError {
    /// A required resource was not present.
    Missing {
        key: ResourceKey,
        path: String,
        required: bool,
    },
    /// Resource bytes were not valid UTF-8.
    InvalidUtf8 {
        key: ResourceKey,
        path: String,
        required: bool,
        details: String,
    },
    /// Resource content failed Fluent parsing.
    Parse {
        key: ResourceKey,
        path: String,
        required: bool,
        details: String,
    },
    /// Resource loading failed in the host asset pipeline.
    Load {
        key: ResourceKey,
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

    /// Constructs an asset-pipeline load error for a resource spec.
    pub fn load(spec: &ModuleResourceSpec, details: impl Into<String>) -> Self {
        Self::Load {
            key: spec.key.clone(),
            path: spec.locale_relative_path.clone(),
            required: spec.required,
            details: details.into(),
        }
    }

    /// Returns the key associated with this failure.
    pub fn key(&self) -> &ResourceKey {
        match self {
            Self::Missing { key, .. }
            | Self::InvalidUtf8 { key, .. }
            | Self::Parse { key, .. }
            | Self::Load { key, .. } => key,
        }
    }

    /// Returns true when this failure affects required readiness.
    pub fn is_required(&self) -> bool {
        match self {
            Self::Missing { required, .. }
            | Self::InvalidUtf8 { required, .. }
            | Self::Parse { required, .. }
            | Self::Load { required, .. } => *required,
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
            Self::Load {
                key,
                path,
                required,
                details,
            } => write!(
                f,
                "failed to load {} resource '{}' at '{}': {}",
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
    fn resource_key_helpers_return_expected_shapes() {
        let key = ResourceKey::new("app/ui");
        assert_eq!(key.as_str(), "app/ui");
        assert_eq!(key.domain(), "app");
        assert_eq!(key.to_string(), "app/ui");
    }

    #[test]
    fn resource_plan_without_namespaces_requires_base_file() {
        let plan = resource_plan_for("app", &[]);
        assert_eq!(
            plan,
            vec![ModuleResourceSpec {
                key: ResourceKey::new("app"),
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
                    key: ResourceKey::new("app"),
                    locale_relative_path: "app.ftl".to_string(),
                    required: false
                },
                ModuleResourceSpec {
                    key: ResourceKey::new("app/ui"),
                    locale_relative_path: "app/ui.ftl".to_string(),
                    required: true
                },
                ModuleResourceSpec {
                    key: ResourceKey::new("app/errors"),
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
        assert_eq!(plan[1].key, ResourceKey::new("app/ui"));
    }

    #[test]
    fn locale_is_ready_requires_all_required_keys() {
        let plan = resource_plan_for("app", &["ui", "errors"]);
        let required = required_resource_keys_from_plan(&plan);
        let optional = optional_resource_keys_from_plan(&plan);

        assert_eq!(optional, HashSet::from([ResourceKey::new("app")]));

        let ready_loaded =
            HashSet::from([ResourceKey::new("app/ui"), ResourceKey::new("app/errors")]);
        assert!(locale_is_ready(&required, &ready_loaded));

        let missing_required = HashSet::from([ResourceKey::new("app/ui")]);
        assert!(!locale_is_ready(&required, &missing_required));
    }

    #[test]
    fn locale_load_report_tracks_errors_and_readiness() {
        let plan = resource_plan_for("app", &["ui"]);
        let mut report = LocaleLoadReport::from_plan(&plan);

        report.mark_loaded(ResourceKey::new("app/ui"));
        report.record_error(ResourceLoadError::load(&plan[0], "file watcher error"));

        assert!(report.is_ready());
        assert_eq!(
            report.required_keys(),
            &HashSet::from([ResourceKey::new("app/ui")])
        );
        assert_eq!(
            report.optional_keys(),
            &HashSet::from([ResourceKey::new("app")])
        );
        assert!(report.loaded_keys().contains(&ResourceKey::new("app/ui")));
        assert_eq!(report.missing_required_keys(), HashSet::new());
    }

    #[test]
    fn validate_module_registry_rejects_duplicates_and_invalid_namespaces() {
        static DUP_LANGUAGE: &[LanguageIdentifier] = &[langid!("en"), langid!("en")];
        static INVALID_NAMESPACES: &[&str] = &["ui", "ui", "", "errors.ftl", "bad/path"];
        static BAD_DATA: ModuleData = ModuleData {
            name: "test-module",
            domain: "test-domain",
            supported_languages: DUP_LANGUAGE,
            namespaces: INVALID_NAMESPACES,
        };
        static DUP_DOMAIN: ModuleData = ModuleData {
            name: "other-module",
            domain: "test-domain",
            supported_languages: SUPPORTED,
            namespaces: &[],
        };

        let errs = validate_module_registry([&DATA, &BAD_DATA, &DUP_DOMAIN])
            .expect_err("validation should fail");
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateModuleName { name } if name == "test-module"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateDomain { domain } if domain == "test-domain"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateSupportedLanguage { module, .. } if module == "test-module"
        )));
        assert!(errs.iter().any(|err| matches!(
            err,
            ModuleRegistryError::DuplicateNamespace { module, namespace } if module == "test-module" && namespace == "ui"
        )));
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
            key: ResourceKey::new("app/ui"),
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
            key: ResourceKey::new("app/ui"),
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
