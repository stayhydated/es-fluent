use super::resource::{ModuleResourceSpec, ResourcePlanError};
use es_fluent_shared::namespace::{NamespacePathError, ResolvedNamespace};
use es_fluent_shared::registry::StaticFluentDomain;
use std::collections::HashSet;
use std::fmt;
use unic_langid::LanguageIdentifier;

/// A package-local Fluent domain and its namespace files.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleDomain {
    /// Domain name from the owning crate's `i18n.toml`.
    pub domain: StaticFluentDomain,
    /// Namespaces configured for this domain.
    pub namespaces: &'static [&'static str],
}

/// Static metadata describing an i18n module.
///
/// This single shape is shared by all managers (embedded, Bevy, and future
/// third-party backends) so module discovery and routing can be standardized.
#[derive(Debug, Eq, PartialEq)]
pub struct ModuleData {
    /// The unique module name (typically crate name).
    pub name: &'static str,
    /// The crate whose `i18n.toml` owns this module's domains.
    pub owner: StaticFluentDomain,
    /// Languages that this module can provide.
    pub supported_languages: &'static [LanguageIdentifier],
    /// Package-local domains and their namespace files.
    pub domains: &'static [ModuleDomain],
}

impl ModuleData {
    /// Returns the global/default canonical resource plan for this module.
    ///
    /// For each domain without namespaces, this plan requires `{domain}.ftl`.
    /// With namespaces, the base file remains optional and every namespace is
    /// required. Managers should prefer
    /// [`crate::I18nModuleRegistration::resource_plan_for_language`] when a
    /// registration provides a sparse per-language manifest plan.
    pub fn resource_plan(&self) -> Vec<ModuleResourceSpec> {
        self.try_resource_plan()
            .expect("module metadata domains are statically validated")
    }

    /// Attempts to return the global/default canonical resource plan.
    ///
    /// Returns invalid namespace metadata as a typed error.
    pub fn try_resource_plan(&self) -> Result<Vec<ModuleResourceSpec>, ResourcePlanError> {
        let mut specs = Vec::new();
        for domain in self.domains {
            specs.extend(
                super::resource::ResourcePlan::for_static_domain(domain.domain, domain.namespaces)?
                    .into_specs(),
            );
        }
        Ok(specs)
    }

    /// Returns whether this module defines the package-local domain.
    pub fn owns_domain(&self, domain: StaticFluentDomain) -> bool {
        self.domains
            .iter()
            .any(|candidate| candidate.domain == domain)
    }
}

/// Validation failures for a discovered module registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleRegistryError {
    /// A module has an empty name.
    EmptyModuleName,
    /// A module has an empty owner.
    EmptyOwner { module: String },
    /// A module name appears more than once.
    DuplicateModuleName { name: String },
    /// An owner appears more than once.
    DuplicateOwner { owner: String },
    /// A module declares the same package-local domain more than once.
    DuplicateDomain { module: String, domain: String },
    /// A module declares the same language more than once.
    DuplicateSupportedLanguage {
        module: String,
        language: LanguageIdentifier,
    },
    /// A module declares the same namespace more than once.
    DuplicateNamespace {
        module: String,
        domain: String,
        namespace: String,
    },
    /// A namespace entry is malformed.
    InvalidNamespace {
        module: String,
        domain: String,
        namespace: String,
        details: NamespacePathError,
    },
}

impl fmt::Display for ModuleRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyModuleName => write!(f, "module name must not be empty"),
            Self::EmptyOwner { module } => {
                write!(f, "module '{}' has an empty owner", module)
            },
            Self::DuplicateModuleName { name } => {
                write!(f, "duplicate module name '{}'", name)
            },
            Self::DuplicateOwner { owner } => {
                write!(f, "duplicate module owner '{}'", owner)
            },
            Self::DuplicateDomain { module, domain } => {
                write!(
                    f,
                    "module '{}' declares duplicate domain '{}'",
                    module, domain
                )
            },
            Self::DuplicateSupportedLanguage { module, language } => write!(
                f,
                "module '{}' declares duplicate language '{}'",
                module, language
            ),
            Self::DuplicateNamespace {
                module,
                domain,
                namespace,
            } => write!(
                f,
                "module '{}' domain '{}' declares duplicate namespace '{}'",
                module, domain, namespace
            ),
            Self::InvalidNamespace {
                module,
                domain,
                namespace,
                details,
            } => write!(
                f,
                "module '{}' domain '{}' has invalid namespace '{}': {}",
                module, domain, namespace, details
            ),
        }
    }
}

impl std::error::Error for ModuleRegistryError {}

/// Validates module metadata discovered through inventory.
///
/// Contract:
/// - `name` and `owner` must be non-empty and globally unique.
/// - `supported_languages` must not contain duplicates.
/// - Domains and namespaces must not contain duplicates within one module.
/// - Namespaces use canonical forward-slash paths such as `ui` or `ui/button`.
/// - Namespace paths must be relative, must not contain `.` or `..` segments,
///   must not have leading or trailing whitespace, and must not include the
///   `.ftl` suffix.
pub fn validate_module_registry<'a>(
    modules: impl IntoIterator<Item = &'a ModuleData>,
) -> Result<(), Vec<ModuleRegistryError>> {
    let mut errors = Vec::new();
    let mut module_names = HashSet::new();
    let mut module_owners = HashSet::new();

    for data in modules {
        if data.name.trim().is_empty() {
            errors.push(ModuleRegistryError::EmptyModuleName);
        } else if !module_names.insert(data.name) {
            errors.push(ModuleRegistryError::DuplicateModuleName {
                name: data.name.to_string(),
            });
        }

        if data.owner.as_str().trim().is_empty() {
            errors.push(ModuleRegistryError::EmptyOwner {
                module: data.name.to_string(),
            });
        } else if !module_owners.insert(data.owner) {
            errors.push(ModuleRegistryError::DuplicateOwner {
                owner: data.owner.as_str().to_string(),
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

        let mut seen_domains = HashSet::new();
        for domain in data.domains {
            if !seen_domains.insert(domain.domain) {
                errors.push(ModuleRegistryError::DuplicateDomain {
                    module: data.name.to_string(),
                    domain: domain.domain.as_str().to_string(),
                });
            }

            let mut seen_namespaces = HashSet::new();
            for namespace in domain.namespaces {
                let namespace = match ResolvedNamespace::new(*namespace) {
                    Ok(namespace) => namespace,
                    Err(details) => {
                        errors.push(ModuleRegistryError::InvalidNamespace {
                            module: data.name.to_string(),
                            domain: domain.domain.as_str().to_string(),
                            namespace: namespace.to_string(),
                            details,
                        });
                        continue;
                    },
                };
                if !seen_namespaces.insert(namespace.clone()) {
                    errors.push(ModuleRegistryError::DuplicateNamespace {
                        module: data.name.to_string(),
                        domain: domain.domain.as_str().to_string(),
                        namespace: namespace.to_string(),
                    });
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    static VALID_MODULE: ModuleData = ModuleData {
        name: "demo-module",
        owner: crate::__macro::static_domain("demo-domain"),
        supported_languages: &[langid!("en"), langid!("fr")],
        domains: &[crate::ModuleDomain {
            domain: crate::__macro::static_domain("demo-domain"),
            namespaces: &["ui", "errors"],
        }],
    };
    static DUPLICATE_MODULES: [&ModuleData; 2] = [
        &ModuleData {
            name: "dup-name",
            owner: crate::__macro::static_domain("dup-domain-a"),
            supported_languages: &[langid!("en"), langid!("en")],
            domains: &[
                crate::ModuleDomain {
                    domain: crate::__macro::static_domain("dup-domain-a"),
                    namespaces: &["ui", "ui"],
                },
                crate::ModuleDomain {
                    domain: crate::__macro::static_domain("dup-domain-a"),
                    namespaces: &[],
                },
            ],
        },
        &ModuleData {
            name: "dup-name",
            owner: crate::__macro::static_domain("dup-domain-a"),
            supported_languages: &[],
            domains: &[crate::ModuleDomain {
                domain: crate::__macro::static_domain("dup-domain-a"),
                namespaces: &["../bad"],
            }],
        },
    ];

    #[test]
    fn module_data_resource_plan_uses_canonical_namespaced_paths() {
        let plan = VALID_MODULE.resource_plan();
        let keys: Vec<_> = plan.iter().map(|spec| spec.key.as_str()).collect();
        let paths: Vec<_> = plan
            .iter()
            .map(|spec| spec.locale_relative_path.as_str())
            .collect();

        assert_eq!(
            keys,
            vec!["demo-domain", "demo-domain/ui", "demo-domain/errors"]
        );
        assert_eq!(
            paths,
            vec![
                "demo-domain.ftl",
                "demo-domain/ui.ftl",
                "demo-domain/errors.ftl",
            ]
        );
        assert!(!plan[0].required);
        assert!(plan[1..].iter().all(|spec| spec.required));
    }

    #[test]
    fn validate_module_registry_accepts_valid_metadata() {
        assert!(validate_module_registry([&VALID_MODULE]).is_ok());
    }

    #[test]
    fn validate_module_registry_reports_duplicates_and_invalid_namespaces() {
        let errors =
            validate_module_registry(DUPLICATE_MODULES).expect_err("invalid registry should fail");

        assert!(errors.contains(&ModuleRegistryError::DuplicateModuleName {
            name: "dup-name".to_string(),
        }));
        assert!(errors.contains(&ModuleRegistryError::DuplicateOwner {
            owner: "dup-domain-a".to_string(),
        }));
        assert!(errors.contains(&ModuleRegistryError::DuplicateDomain {
            module: "dup-name".to_string(),
            domain: "dup-domain-a".to_string(),
        }));
        assert!(
            errors.contains(&ModuleRegistryError::DuplicateSupportedLanguage {
                module: "dup-name".to_string(),
                language: langid!("en"),
            })
        );
        assert!(errors.contains(&ModuleRegistryError::DuplicateNamespace {
            module: "dup-name".to_string(),
            domain: "dup-domain-a".to_string(),
            namespace: "ui".to_string(),
        }));
        assert!(errors.iter().any(|error| matches!(
            error,
            ModuleRegistryError::InvalidNamespace { module, namespace, .. }
                if module == "dup-name" && namespace == "../bad"
        )));
    }

    #[test]
    fn module_registry_error_messages_are_descriptive() {
        let cases = [
            (
                ModuleRegistryError::EmptyModuleName,
                "module name must not be empty",
            ),
            (
                ModuleRegistryError::EmptyOwner {
                    module: "demo".to_string(),
                },
                "module 'demo' has an empty owner",
            ),
            (
                ModuleRegistryError::DuplicateModuleName {
                    name: "demo".to_string(),
                },
                "duplicate module name 'demo'",
            ),
            (
                ModuleRegistryError::DuplicateDomain {
                    module: "demo".to_string(),
                    domain: "demo".to_string(),
                },
                "module 'demo' declares duplicate domain 'demo'",
            ),
            (
                ModuleRegistryError::DuplicateSupportedLanguage {
                    module: "demo".to_string(),
                    language: langid!("en"),
                },
                "module 'demo' declares duplicate language 'en'",
            ),
            (
                ModuleRegistryError::DuplicateNamespace {
                    module: "demo".to_string(),
                    domain: "demo".to_string(),
                    namespace: "ui".to_string(),
                },
                "module 'demo' domain 'demo' declares duplicate namespace 'ui'",
            ),
            (
                ModuleRegistryError::InvalidNamespace {
                    module: "demo".to_string(),
                    domain: "demo".to_string(),
                    namespace: "../ui".to_string(),
                    details: NamespacePathError::CurrentOrParentSegment,
                },
                "module 'demo' domain 'demo' has invalid namespace '../ui': namespace path must not contain '.' or '..' segments",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn static_module_descriptor_returns_original_module_data() {
        let descriptor = StaticModuleDescriptor::new(&VALID_MODULE);
        assert_eq!(descriptor.data(), &VALID_MODULE);
    }
}
