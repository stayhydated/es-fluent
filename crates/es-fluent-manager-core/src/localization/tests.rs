use super::*;
use crate::asset_localization::{I18nModuleDescriptor, ModuleData, StaticModuleDescriptor};
use fluent_bundle::FluentResource;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use unic_langid::langid;

static EXPLICIT_RUNTIME_CREATE_CALLS: AtomicUsize = AtomicUsize::new(0);
static SELECT_OK_CALLS: AtomicUsize = AtomicUsize::new(0);
static SELECT_ERR_CALLS: AtomicUsize = AtomicUsize::new(0);
static MODULE_OK_DATA: ModuleData = ModuleData {
    name: "module-ok",
    domain: "module-ok",
    supported_languages: &[],
    namespaces: &[],
};
static MODULE_ERR_DATA: ModuleData = ModuleData {
    name: "module-err",
    domain: "module-err",
    supported_languages: &[],
    namespaces: &[],
};
static STATEFUL_SUCCESS_DATA: ModuleData = ModuleData {
    name: "stateful-success",
    domain: "stateful-success",
    supported_languages: &[],
    namespaces: &[],
};
static STATEFUL_FAIL_DATA: ModuleData = ModuleData {
    name: "stateful-fail",
    domain: "stateful-fail",
    supported_languages: &[],
    namespaces: &[],
};
static EXPLICIT_RUNTIME_DATA: ModuleData = ModuleData {
    name: "explicit-runtime",
    domain: "explicit-runtime",
    supported_languages: &[],
    namespaces: &[],
};
static FILTER_MODULE_DATA: ModuleData = ModuleData {
    name: "filter-module",
    domain: "filter-domain",
    supported_languages: &[],
    namespaces: &[],
};
static FILTER_DUP_NAME_DATA: ModuleData = ModuleData {
    name: "filter-module",
    domain: "filter-domain-b",
    supported_languages: &[],
    namespaces: &[],
};
static FILTER_DUP_DOMAIN_DATA: ModuleData = ModuleData {
    name: "filter-module-b",
    domain: "filter-domain",
    supported_languages: &[],
    namespaces: &[],
};
static FILTER_EXACT_DUP_DATA: ModuleData = ModuleData {
    name: "filter-exact-module",
    domain: "filter-exact-domain",
    supported_languages: &[],
    namespaces: &[],
};
static FILTER_EXACT_DUP_RUNTIME_MISMATCH_DATA: ModuleData = ModuleData {
    name: "filter-exact-module",
    domain: "filter-exact-domain",
    supported_languages: &[langid!("en")],
    namespaces: &["ui"],
};
static FILTER_INVALID_NAMESPACE_DATA: ModuleData = ModuleData {
    name: "filter-invalid-namespace",
    domain: "filter-invalid-namespace",
    supported_languages: &[],
    namespaces: &[" ../escape "],
};
static FILTER_DUP_LANGUAGE_DATA: ModuleData = ModuleData {
    name: "filter-dup-language",
    domain: "filter-dup-language",
    supported_languages: &[langid!("en"), langid!("en")],
    namespaces: &[],
};
static FILTER_DESCRIPTOR: StaticModuleDescriptor = StaticModuleDescriptor::new(&FILTER_MODULE_DATA);
static FILTER_DUP_NAME_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_DUP_NAME_DATA);
static FILTER_DUP_DOMAIN_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_DUP_DOMAIN_DATA);
static FILTER_EXACT_DUP_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_EXACT_DUP_DATA);
static FILTER_EXACT_DUP_DESCRIPTOR_TWO: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_EXACT_DUP_DATA);
static FILTER_INVALID_NAMESPACE_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_INVALID_NAMESPACE_DATA);
static FILTER_DUP_LANGUAGE_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_DUP_LANGUAGE_DATA);

struct ModuleOk;
struct ModuleErr;
struct StatefulSuccessModule;
struct StatefulFailModule;
struct FilterRuntimeModule;
struct FilterRuntimeModuleTwo;
struct FilterRuntimeMismatchModule;
struct ExplicitRuntimeRegistration;

struct LocalizerOk;
struct LocalizerErr;
struct FilterRuntimeLocalizer;
struct StatefulSuccessLocalizer {
    selected: RwLock<Option<String>>,
}
struct StatefulFailLocalizer;

impl Localizer for LocalizerOk {
    fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        SELECT_OK_CALLS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        match id {
            "from-ok" => Some("ok-value".to_string()),
            "shared-id" => Some("ok-shared".to_string()),
            _ => None,
        }
    }
}

impl Localizer for LocalizerErr {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        SELECT_ERR_CALLS.fetch_add(1, Ordering::Relaxed);
        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        match id {
            "from-err" => Some("err-value".to_string()),
            "shared-id" => Some("err-shared".to_string()),
            _ => None,
        }
    }
}

impl Localizer for FilterRuntimeLocalizer {
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

impl StatefulSuccessLocalizer {
    fn new(selected: Option<&str>) -> Self {
        Self {
            selected: RwLock::new(selected.map(ToOwned::to_owned)),
        }
    }
}

impl Localizer for StatefulSuccessLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        *self.selected.write().expect("lock poisoned") = Some(lang.to_string());
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (id == "selected-language")
            .then(|| self.selected.read().expect("lock poisoned").clone())
            .flatten()
    }
}

impl Localizer for StatefulFailLocalizer {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        Err(LocalizationError::LanguageNotSupported(lang.clone()))
    }

    fn localize<'a>(
        &self,
        _id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        None
    }
}

impl I18nModuleDescriptor for ModuleOk {
    fn data(&self) -> &'static ModuleData {
        &MODULE_OK_DATA
    }
}

impl I18nModule for ModuleOk {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(LocalizerOk)
    }
}

impl I18nModuleDescriptor for ModuleErr {
    fn data(&self) -> &'static ModuleData {
        &MODULE_ERR_DATA
    }
}

impl I18nModule for ModuleErr {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(LocalizerErr)
    }
}

impl I18nModuleDescriptor for FilterRuntimeModule {
    fn data(&self) -> &'static ModuleData {
        &FILTER_EXACT_DUP_DATA
    }
}

impl I18nModule for FilterRuntimeModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(FilterRuntimeLocalizer)
    }
}

impl I18nModuleDescriptor for FilterRuntimeModuleTwo {
    fn data(&self) -> &'static ModuleData {
        &FILTER_EXACT_DUP_DATA
    }
}

impl I18nModule for FilterRuntimeModuleTwo {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(FilterRuntimeLocalizer)
    }
}

impl I18nModuleDescriptor for FilterRuntimeMismatchModule {
    fn data(&self) -> &'static ModuleData {
        &FILTER_EXACT_DUP_RUNTIME_MISMATCH_DATA
    }
}

impl I18nModule for FilterRuntimeMismatchModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(FilterRuntimeLocalizer)
    }
}

impl I18nModuleDescriptor for StatefulSuccessModule {
    fn data(&self) -> &'static ModuleData {
        &STATEFUL_SUCCESS_DATA
    }
}

impl I18nModule for StatefulSuccessModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(StatefulSuccessLocalizer::new(None))
    }
}

impl I18nModuleDescriptor for StatefulFailModule {
    fn data(&self) -> &'static ModuleData {
        &STATEFUL_FAIL_DATA
    }
}

impl I18nModule for StatefulFailModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(StatefulFailLocalizer)
    }
}

impl I18nModuleDescriptor for ExplicitRuntimeRegistration {
    fn data(&self) -> &'static ModuleData {
        &EXPLICIT_RUNTIME_DATA
    }
}

impl I18nModuleRegistration for ExplicitRuntimeRegistration {
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        EXPLICIT_RUNTIME_CREATE_CALLS.fetch_add(1, Ordering::Relaxed);
        Some(Box::new(FilterRuntimeLocalizer))
    }

    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::RuntimeLocalizer
    }
}

static MODULE_OK: ModuleOk = ModuleOk;
static MODULE_ERR: ModuleErr = ModuleErr;
static STATEFUL_SUCCESS_MODULE: StatefulSuccessModule = StatefulSuccessModule;
static STATEFUL_FAIL_MODULE: StatefulFailModule = StatefulFailModule;
static FILTER_RUNTIME_MODULE: FilterRuntimeModule = FilterRuntimeModule;
static FILTER_RUNTIME_MODULE_TWO: FilterRuntimeModuleTwo = FilterRuntimeModuleTwo;
static FILTER_RUNTIME_MISMATCH_MODULE: FilterRuntimeMismatchModule = FilterRuntimeMismatchModule;
static EXPLICIT_RUNTIME_REGISTRATION: ExplicitRuntimeRegistration = ExplicitRuntimeRegistration;

inventory::submit! {
    &MODULE_OK as &dyn I18nModuleRegistration
}

inventory::submit! {
    &MODULE_ERR as &dyn I18nModuleRegistration
}

#[test]
fn manager_select_language_returns_error_when_any_module_fails() {
    let ok_before = SELECT_OK_CALLS.load(Ordering::Relaxed);
    let err_before = SELECT_ERR_CALLS.load(Ordering::Relaxed);

    let manager = FluentManager::new_with_discovered_modules();
    let err = manager
        .select_language(&langid!("en-US"))
        .expect_err("partial language selection should fail");

    assert!(SELECT_OK_CALLS.load(Ordering::Relaxed) > ok_before);
    assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) > err_before);
    assert!(matches!(err, LocalizationError::LanguageNotSupported(_)));
    assert_eq!(manager.localize("from-ok", None), None);
}

#[test]
fn manager_try_new_with_discovered_modules_succeeds_for_clean_inventory() {
    let manager = FluentManager::try_new_with_discovered_modules()
        .expect("current test inventory should pass strict discovery");
    assert!(!manager.modules.is_empty());
}

#[test]
fn manager_localize_returns_first_matching_message() {
    let manager = FluentManager {
        modules: Vec::new(),
        localizers: RwLock::new(vec![
            (&MODULE_OK_DATA, Box::new(LocalizerOk)),
            (&MODULE_ERR_DATA, Box::new(LocalizerErr)),
        ]),
    };
    assert_eq!(
        manager.localize("from-ok", None),
        Some("ok-value".to_string())
    );
    assert_eq!(
        manager.localize("from-err", None),
        Some("err-value".to_string())
    );
    assert_eq!(
        manager.localize("shared-id", None),
        Some("ok-shared".to_string())
    );
    assert_eq!(
        manager.localize_in_domain("module-err", "shared-id", None),
        Some("err-shared".to_string())
    );
    assert_eq!(manager.localize("missing", None), None);
}

#[test]
fn manager_select_language_with_only_failing_localizers_returns_error() {
    let err_before = SELECT_ERR_CALLS.load(Ordering::Relaxed);

    let manager = FluentManager {
        modules: vec![&MODULE_ERR as &dyn I18nModuleRegistration],
        localizers: RwLock::default(),
    };
    let err = manager
        .select_language(&langid!("en-US"))
        .expect_err("no successful localizer should return an error");

    assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) > err_before);
    assert!(matches!(err, LocalizationError::LanguageNotSupported(_)));
}

#[test]
fn manager_keeps_previous_localizers_when_selection_fails() {
    let manager = FluentManager {
        modules: vec![
            &STATEFUL_SUCCESS_MODULE as &dyn I18nModuleRegistration,
            &STATEFUL_FAIL_MODULE as &dyn I18nModuleRegistration,
        ],
        localizers: RwLock::new(vec![(
            &STATEFUL_SUCCESS_DATA,
            Box::new(StatefulSuccessLocalizer::new(Some("en-US"))),
        )]),
    };

    let err = manager
        .select_language(&langid!("fr"))
        .expect_err("failed selection should not replace the active localizers");

    assert!(matches!(err, LocalizationError::LanguageNotSupported(_)));
    assert_eq!(
        manager.localize("selected-language", None),
        Some("en-US".to_string())
    );
}

#[test]
fn build_sync_bundle_reports_resource_add_errors() {
    let lang = langid!("en-US");
    let first = Arc::new(FluentResource::try_new("hello = first".to_string()).expect("valid ftl"));
    let duplicate =
        Arc::new(FluentResource::try_new("hello = second".to_string()).expect("valid ftl"));

    let (bundle, add_errors) = build_sync_bundle(&lang, vec![first, duplicate]);
    assert!(!add_errors.is_empty());

    let (localized, _format_errors) =
        localize_with_bundle(&bundle, "hello", None).expect("message should exist");
    assert_eq!(localized, "first");
}

#[test]
fn filter_module_registry_skips_duplicate_name_and_domain() {
    let filtered = filter_module_registry([
        &FILTER_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_DUP_NAME_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_DUP_DOMAIN_DESCRIPTOR as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].data().name, "filter-module");
}

#[test]
fn filter_module_registry_prefers_runtime_localizer_for_exact_duplicate_identity() {
    let filtered = filter_module_registry([
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_RUNTIME_MODULE as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].create_localizer().is_some());
}

#[test]
fn filter_module_registry_keeps_runtime_localizer_when_metadata_duplicate_follows() {
    let filtered = filter_module_registry([
        &FILTER_RUNTIME_MODULE as &dyn I18nModuleRegistration,
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].create_localizer().is_some());
}

#[test]
fn filter_module_registry_uses_explicit_registration_kind_without_constructing_localizers() {
    EXPLICIT_RUNTIME_CREATE_CALLS.store(0, Ordering::Relaxed);

    let filtered =
        filter_module_registry([&EXPLICIT_RUNTIME_REGISTRATION as &dyn I18nModuleRegistration]);

    assert_eq!(filtered.len(), 1);
    assert_eq!(EXPLICIT_RUNTIME_CREATE_CALLS.load(Ordering::Relaxed), 0);
}

#[test]
fn filter_module_registry_skips_entries_with_invalid_metadata() {
    let filtered = filter_module_registry([
        &FILTER_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_INVALID_NAMESPACE_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_DUP_LANGUAGE_DESCRIPTOR as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].data().name, "filter-module");
}

#[test]
fn try_filter_module_registry_allows_exact_runtime_and_metadata_pairing() {
    let filtered = try_filter_module_registry([
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_RUNTIME_MODULE as &dyn I18nModuleRegistration,
    ])
    .expect("metadata plus runtime for one exact identity should remain valid");

    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].create_localizer().is_some());
}

#[test]
fn try_filter_module_registry_rejects_duplicate_metadata_only_registrations() {
    let errors = match try_filter_module_registry([
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_EXACT_DUP_DESCRIPTOR_TWO as &dyn I18nModuleRegistration,
    ]) {
        Ok(_) => panic!("strict discovery should reject repeated metadata-only registrations"),
        Err(errors) => errors,
    };

    assert!(errors.iter().any(|error| {
        matches!(
            error,
            ModuleDiscoveryError::DuplicateModuleRegistration {
                name,
                domain,
                kind: ModuleRegistrationKind::MetadataOnly,
                count: 2,
            } if name == "filter-exact-module" && domain == "filter-exact-domain"
        )
    }));
}

#[test]
fn try_filter_module_registry_rejects_duplicate_runtime_registrations() {
    let errors = match try_filter_module_registry([
        &FILTER_RUNTIME_MODULE as &dyn I18nModuleRegistration,
        &FILTER_RUNTIME_MODULE_TWO as &dyn I18nModuleRegistration,
    ]) {
        Ok(_) => panic!("strict discovery should reject repeated runtime registrations"),
        Err(errors) => errors,
    };

    assert!(errors.iter().any(|error| {
        matches!(
            error,
            ModuleDiscoveryError::DuplicateModuleRegistration {
                name,
                domain,
                kind: ModuleRegistrationKind::RuntimeLocalizer,
                count: 2,
            } if name == "filter-exact-module" && domain == "filter-exact-domain"
        )
    }));
}

#[test]
fn try_filter_module_registry_rejects_mismatched_metadata_runtime_pairing() {
    let errors = match try_filter_module_registry([
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_RUNTIME_MISMATCH_MODULE as &dyn I18nModuleRegistration,
    ]) {
        Ok(_) => panic!("strict discovery should reject mismatched paired metadata"),
        Err(errors) => errors,
    };

    assert!(errors.iter().any(|error| {
        matches!(
            error,
            ModuleDiscoveryError::InconsistentModuleMetadata { name, domain }
                if name == "filter-exact-module" && domain == "filter-exact-domain"
        )
    }));
}
