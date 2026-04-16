use super::resource::{ModuleResourceSpec, resource_plan_for};
use es_fluent_shared::namespace::validate_namespace_path;
use std::collections::HashSet;
use std::fmt;
use unic_langid::LanguageIdentifier;

/// Static metadata describing an i18n module.
///
/// This single shape is shared by all managers (embedded, Bevy, and future
/// third-party backends) so module discovery and routing can be standardized.
#[derive(Debug, Eq, PartialEq)]
pub struct ModuleData {
    /// The unique module name (typically crate name).
    pub name: &'static str,
    /// The Fluent domain for this module.
    pub domain: &'static str,
    /// Languages that this module can provide.
    pub supported_languages: &'static [LanguageIdentifier],
    /// Namespaces used by the module (e.g., "ui", "ui/button").
    /// If empty, only the main `{domain}.ftl` file is used.
    /// If non-empty, only the configured namespace files are canonical
    /// resources for the domain.
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
/// - Namespaces use canonical forward-slash paths such as `ui` or `ui/button`.
/// - Namespace paths must be relative, must not contain `.` or `..` segments,
///   must not have leading or trailing whitespace, and must not include the
///   `.ftl` suffix.
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
            if let Err(details) = validate_namespace_path(namespace) {
                errors.push(ModuleRegistryError::InvalidNamespace {
                    module: data.name.to_string(),
                    namespace: namespace.to_string(),
                    details,
                });
                continue;
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
