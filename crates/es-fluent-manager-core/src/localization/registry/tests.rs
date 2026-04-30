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
static REGISTRY_METADATA: StaticModuleDescriptor = StaticModuleDescriptor::new(&REGISTRY_TEST_DATA);
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

    let invalid =
        match try_filter_module_registry(
            [&REGISTRY_INVALID_METADATA as &dyn I18nModuleRegistration],
        ) {
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
