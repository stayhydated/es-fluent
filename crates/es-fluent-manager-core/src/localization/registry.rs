use super::I18nModuleRegistration;
use crate::asset_localization::ModuleData;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleRegistrationKind {
    MetadataOnly,
    RuntimeLocalizer,
}

impl std::fmt::Display for ModuleRegistrationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MetadataOnly => f.write_str("metadata-only"),
            Self::RuntimeLocalizer => f.write_str("runtime-localizer"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleDiscoveryError {
    InvalidMetadata(crate::asset_localization::ModuleRegistryError),
    InconsistentModuleMetadata {
        name: String,
        domain: String,
    },
    DuplicateModuleRegistration {
        name: String,
        domain: String,
        kind: ModuleRegistrationKind,
        count: usize,
    },
}

impl std::fmt::Display for ModuleDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetadata(error) => write!(f, "{error}"),
            Self::InconsistentModuleMetadata { name, domain } => write!(
                f,
                "module '{name}' (domain '{domain}') has mismatched metadata between registrations",
            ),
            Self::DuplicateModuleRegistration {
                name,
                domain,
                kind,
                count,
            } => write!(
                f,
                "module '{name}' (domain '{domain}') has {count} duplicate {kind} registrations",
            ),
        }
    }
}

impl std::error::Error for ModuleDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidMetadata(error) => Some(error),
            Self::InconsistentModuleMetadata { .. } => None,
            Self::DuplicateModuleRegistration { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RegistrationCounts {
    metadata_only: usize,
    runtime_localizer: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct IdentityInspection {
    chosen_data: Option<&'static ModuleData>,
    metadata_only: Option<&'static ModuleData>,
    runtime_localizer: Option<&'static ModuleData>,
    counts: RegistrationCounts,
}

fn validate_single_module_data(data: &'static ModuleData, errors: &mut Vec<ModuleDiscoveryError>) {
    if let Err(validation_errors) = crate::asset_localization::validate_module_registry([data]) {
        errors.extend(
            validation_errors
                .into_iter()
                .map(ModuleDiscoveryError::InvalidMetadata),
        );
    }
}

/// Validates discovered registrations strictly and returns either the validated
/// module list or the collected registry/discovery errors.
///
/// Strict validation still allows one metadata-only registration plus one
/// runtime-localizer registration for the same exact (`name`, `domain`)
/// identity, because that pairing is used intentionally by some integrations.
/// It rejects repeated registrations of the same kind for one identity and all
/// metadata validation failures reported by `validate_module_registry()`. Exact
/// metadata/runtime pairs are preserved so integrations can filter the
/// registration kind they need.
pub fn try_filter_module_registry(
    modules: impl IntoIterator<Item = &'static dyn I18nModuleRegistration>,
) -> Result<Vec<&'static dyn I18nModuleRegistration>, Vec<ModuleDiscoveryError>> {
    let modules = modules.into_iter().collect::<Vec<_>>();
    let inspections = inspect_module_registry(&modules);
    let discovered_data = inspections
        .values()
        .filter_map(|inspection| inspection.chosen_data)
        .collect::<Vec<_>>();

    let mut errors = Vec::new();
    if let Err(validation_errors) =
        crate::asset_localization::validate_module_registry(discovered_data.iter().copied())
    {
        errors.extend(
            validation_errors
                .into_iter()
                .map(ModuleDiscoveryError::InvalidMetadata),
        );
    }

    for ((name, domain), inspection) in inspections {
        if let (Some(metadata_only), Some(runtime_localizer)) =
            (inspection.metadata_only, inspection.runtime_localizer)
            && metadata_only != runtime_localizer
        {
            errors.push(ModuleDiscoveryError::InconsistentModuleMetadata {
                name: name.to_string(),
                domain: domain.to_string(),
            });

            if inspection.chosen_data != Some(metadata_only) {
                validate_single_module_data(metadata_only, &mut errors);
            }
            if inspection.chosen_data != Some(runtime_localizer) {
                validate_single_module_data(runtime_localizer, &mut errors);
            }
        }

        let counts = inspection.counts;
        if counts.metadata_only > 1 {
            errors.push(ModuleDiscoveryError::DuplicateModuleRegistration {
                name: name.to_string(),
                domain: domain.to_string(),
                kind: ModuleRegistrationKind::MetadataOnly,
                count: counts.metadata_only,
            });
        }
        if counts.runtime_localizer > 1 {
            errors.push(ModuleDiscoveryError::DuplicateModuleRegistration {
                name: name.to_string(),
                domain: domain.to_string(),
                kind: ModuleRegistrationKind::RuntimeLocalizer,
                count: counts.runtime_localizer,
            });
        }
    }

    if errors.is_empty() {
        Ok(modules)
    } else {
        Err(errors)
    }
}

fn inspect_module_registry(
    modules: &[&'static dyn I18nModuleRegistration],
) -> HashMap<(&'static str, &'static str), IdentityInspection> {
    let mut inspections: HashMap<(&'static str, &'static str), IdentityInspection> = HashMap::new();

    for module in modules {
        let data = module.data();
        let inspection = inspections.entry((data.name, data.domain)).or_default();
        inspection.chosen_data.get_or_insert(data);

        match module.registration_kind() {
            ModuleRegistrationKind::MetadataOnly => {
                inspection.counts.metadata_only += 1;
                inspection.metadata_only.get_or_insert(data);
            },
            ModuleRegistrationKind::RuntimeLocalizer => {
                inspection.counts.runtime_localizer += 1;
                inspection.runtime_localizer.get_or_insert(data);
            },
        }
    }

    inspections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_localization::{I18nModuleDescriptor, StaticModuleDescriptor};
    use crate::localization::{I18nModule, LocalizationError, Localizer};
    use fluent_bundle::FluentValue;
    use std::collections::HashMap;
    use std::error::Error as _;
    use unic_langid::{LanguageIdentifier, langid};

    static REGISTRY_TEST_LANGUAGES: &[LanguageIdentifier] = &[langid!("en")];
    static REGISTRY_TEST_DATA: ModuleData = ModuleData {
        name: "registry-test",
        domain: "registry-domain",
        supported_languages: REGISTRY_TEST_LANGUAGES,
        namespaces: &[],
    };
    static REGISTRY_INVALID_DATA: ModuleData = ModuleData {
        name: "registry-invalid",
        domain: "registry-invalid",
        supported_languages: &[],
        namespaces: &[" ../escape "],
    };
    static REGISTRY_METADATA: StaticModuleDescriptor =
        StaticModuleDescriptor::new(&REGISTRY_TEST_DATA);
    static REGISTRY_METADATA_TWO: StaticModuleDescriptor =
        StaticModuleDescriptor::new(&REGISTRY_TEST_DATA);
    static REGISTRY_INVALID_METADATA: StaticModuleDescriptor =
        StaticModuleDescriptor::new(&REGISTRY_INVALID_DATA);

    struct RegistryRuntimeModule;
    struct RegistryRuntimeModuleTwo;
    struct RegistryNoopLocalizer;

    impl Localizer for RegistryNoopLocalizer {
        fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            Ok(())
        }

        fn localize<'a>(
            &self,
            _id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            None
        }
    }

    impl I18nModuleDescriptor for RegistryRuntimeModule {
        fn data(&self) -> &'static ModuleData {
            &REGISTRY_TEST_DATA
        }
    }

    impl I18nModule for RegistryRuntimeModule {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(RegistryNoopLocalizer)
        }
    }

    impl I18nModuleDescriptor for RegistryRuntimeModuleTwo {
        fn data(&self) -> &'static ModuleData {
            &REGISTRY_TEST_DATA
        }
    }

    impl I18nModule for RegistryRuntimeModuleTwo {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(RegistryNoopLocalizer)
        }
    }

    static REGISTRY_RUNTIME: RegistryRuntimeModule = RegistryRuntimeModule;
    static REGISTRY_RUNTIME_TWO: RegistryRuntimeModuleTwo = RegistryRuntimeModuleTwo;

    #[test]
    fn inspect_module_registry_records_counts_for_each_registration_kind() {
        let modules: [&'static dyn I18nModuleRegistration; 2] = [
            &REGISTRY_METADATA as &dyn I18nModuleRegistration,
            &REGISTRY_RUNTIME as &dyn I18nModuleRegistration,
        ];

        let inspections = inspect_module_registry(&modules);
        let inspection = inspections
            .get(&("registry-test", "registry-domain"))
            .expect("module identity should be inspected");

        assert_eq!(inspection.chosen_data, Some(&REGISTRY_TEST_DATA));
        assert_eq!(inspection.metadata_only, Some(&REGISTRY_TEST_DATA));
        assert_eq!(inspection.runtime_localizer, Some(&REGISTRY_TEST_DATA));
        assert_eq!(
            inspection.counts,
            RegistrationCounts {
                metadata_only: 1,
                runtime_localizer: 1,
            }
        );
    }

    #[test]
    fn try_filter_module_registry_allows_empty_and_exact_metadata_runtime_pair() {
        let empty: [&'static dyn I18nModuleRegistration; 0] = [];
        assert!(
            try_filter_module_registry(empty)
                .expect("empty registry is valid")
                .is_empty()
        );

        let filtered = try_filter_module_registry([
            &REGISTRY_METADATA as &dyn I18nModuleRegistration,
            &REGISTRY_RUNTIME as &dyn I18nModuleRegistration,
        ])
        .expect("exact metadata and runtime registrations should be valid");

        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn try_filter_module_registry_reports_duplicate_registration_kinds() {
        let metadata_errors = match try_filter_module_registry([
            &REGISTRY_METADATA as &dyn I18nModuleRegistration,
            &REGISTRY_METADATA_TWO as &dyn I18nModuleRegistration,
        ]) {
            Ok(_) => panic!("duplicate metadata registrations should fail"),
            Err(errors) => errors,
        };
        assert!(metadata_errors.iter().any(|error| {
            matches!(
                error,
                ModuleDiscoveryError::DuplicateModuleRegistration {
                    kind: ModuleRegistrationKind::MetadataOnly,
                    count: 2,
                    ..
                }
            )
        }));

        let runtime_errors = match try_filter_module_registry([
            &REGISTRY_RUNTIME as &dyn I18nModuleRegistration,
            &REGISTRY_RUNTIME_TWO as &dyn I18nModuleRegistration,
        ]) {
            Ok(_) => panic!("duplicate runtime registrations should fail"),
            Err(errors) => errors,
        };
        assert!(runtime_errors.iter().any(|error| {
            matches!(
                error,
                ModuleDiscoveryError::DuplicateModuleRegistration {
                    kind: ModuleRegistrationKind::RuntimeLocalizer,
                    count: 2,
                    ..
                }
            )
        }));
    }

    #[test]
    fn validate_single_module_data_wraps_registry_validation_errors() {
        let mut errors = Vec::new();
        validate_single_module_data(&REGISTRY_INVALID_DATA, &mut errors);

        let invalid = errors
            .iter()
            .find(|error| matches!(error, ModuleDiscoveryError::InvalidMetadata(_)))
            .expect("invalid metadata should be wrapped");
        assert!(invalid.source().is_some());
        assert!(invalid.to_string().contains("namespace"));
    }

    #[test]
    fn module_discovery_errors_format_diagnostics_and_sources() {
        assert_eq!(
            ModuleRegistrationKind::MetadataOnly.to_string(),
            "metadata-only"
        );
        assert_eq!(
            ModuleRegistrationKind::RuntimeLocalizer.to_string(),
            "runtime-localizer"
        );

        let invalid = match try_filter_module_registry([
            &REGISTRY_INVALID_METADATA as &dyn I18nModuleRegistration
        ]) {
            Ok(_) => panic!("invalid metadata should fail"),
            Err(mut errors) => errors.remove(0),
        };
        assert!(invalid.source().is_some());

        let inconsistent = ModuleDiscoveryError::InconsistentModuleMetadata {
            name: "name".to_string(),
            domain: "domain".to_string(),
        };
        assert_eq!(
            inconsistent.to_string(),
            "module 'name' (domain 'domain') has mismatched metadata between registrations"
        );
        assert!(inconsistent.source().is_none());

        let duplicate = ModuleDiscoveryError::DuplicateModuleRegistration {
            name: "name".to_string(),
            domain: "domain".to_string(),
            kind: ModuleRegistrationKind::RuntimeLocalizer,
            count: 3,
        };
        assert_eq!(
            duplicate.to_string(),
            "module 'name' (domain 'domain') has 3 duplicate runtime-localizer registrations"
        );
        assert!(duplicate.source().is_none());
    }
}
