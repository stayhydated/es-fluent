use super::*;
use crate::asset_localization::{I18nModuleDescriptor, ModuleData, StaticModuleDescriptor};
use crate::localization::manager::{format_module_support, format_supported_languages};
use fluent_bundle::FluentResource;
use parking_lot::RwLock;
use serial_test::serial;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
static HARD_FAIL_DATA: ModuleData = ModuleData {
    name: "hard-fail",
    domain: "hard-fail",
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
static DIAGNOSTIC_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[
    langid!("en"),
    langid!("fr"),
    langid!("de"),
    langid!("es"),
    langid!("it"),
    langid!("ja"),
    langid!("zh"),
];
static DIAGNOSTIC_MODULE_DATA: ModuleData = ModuleData {
    name: "diagnostic-module",
    domain: "diagnostic-domain",
    supported_languages: DIAGNOSTIC_SUPPORTED_LANGUAGES,
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
struct HardFailModule;
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
struct HardFailLocalizer;

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
        *self.selected.write() = Some(lang.to_string());
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (id == "selected-language")
            .then(|| self.selected.read().clone())
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

impl Localizer for HardFailLocalizer {
    fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        Err(io::Error::other("hard failure").into())
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

impl I18nModuleDescriptor for HardFailModule {
    fn data(&self) -> &'static ModuleData {
        &HARD_FAIL_DATA
    }
}

impl I18nModule for HardFailModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(HardFailLocalizer)
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
static HARD_FAIL_MODULE: HardFailModule = HardFailModule;
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
fn manager_select_language_best_effort_skips_unsupported_modules_when_any_module_succeeds() {
    let ok_before = SELECT_OK_CALLS.load(Ordering::Relaxed);
    let err_before = SELECT_ERR_CALLS.load(Ordering::Relaxed);

    let manager = FluentManager::new_with_discovered_modules();
    manager
        .select_language(&langid!("en-US"))
        .expect("best-effort language selection should keep supporting modules active");

    assert!(SELECT_OK_CALLS.load(Ordering::Relaxed) > ok_before);
    assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) > err_before);
    assert_eq!(
        manager.localize("from-ok", None),
        Some("ok-value".to_string())
    );
    assert_eq!(manager.localize("from-err", None), None);
}

#[test]
fn manager_select_language_strict_returns_error_when_any_module_fails() {
    let ok_before = SELECT_OK_CALLS.load(Ordering::Relaxed);
    let err_before = SELECT_ERR_CALLS.load(Ordering::Relaxed);

    let manager = FluentManager::new_with_discovered_modules();
    let err = manager
        .select_language_strict(&langid!("en-US"))
        .expect_err("strict language selection should fail on partial support");

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
#[serial(explicit_runtime_create_calls)]
fn registration_runtime_support_defaults_match_registration_kind() {
    let metadata_only = StaticModuleDescriptor::new(&FILTER_MODULE_DATA);
    assert_eq!(
        metadata_only.registration_kind(),
        ModuleRegistrationKind::MetadataOnly
    );
    assert!(!metadata_only.supports_runtime_localization());
    assert!(metadata_only.create_localizer().is_none());
    assert_eq!(
        metadata_only.resource_plan_for_language(&langid!("en")),
        None
    );

    let runtime_module = &MODULE_OK as &dyn I18nModuleRegistration;
    assert_eq!(
        runtime_module.registration_kind(),
        ModuleRegistrationKind::RuntimeLocalizer
    );
    assert!(runtime_module.supports_runtime_localization());
    assert!(runtime_module.create_localizer().is_some());
    assert_eq!(
        runtime_module.resource_plan_for_language(&langid!("en")),
        None
    );

    EXPLICIT_RUNTIME_CREATE_CALLS.store(0, Ordering::Relaxed);
    assert!(EXPLICIT_RUNTIME_REGISTRATION.supports_runtime_localization());
    assert!(EXPLICIT_RUNTIME_REGISTRATION.create_localizer().is_some());
    assert_eq!(EXPLICIT_RUNTIME_CREATE_CALLS.load(Ordering::Relaxed), 1);
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
fn manager_select_language_returns_error_on_non_unsupported_failure() {
    let manager = FluentManager {
        modules: vec![
            &STATEFUL_SUCCESS_MODULE as &dyn I18nModuleRegistration,
            &HARD_FAIL_MODULE as &dyn I18nModuleRegistration,
        ],
        localizers: RwLock::default(),
    };

    let err = manager
        .select_language(&langid!("fr"))
        .expect_err("unexpected runtime-localizer failures should still abort");

    assert!(matches!(err, LocalizationError::IoError(_)));
    assert_eq!(manager.localize("selected-language", None), None);
}

#[test]
fn manager_keeps_previous_localizers_when_strict_selection_fails() {
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
        .select_language_strict(&langid!("fr"))
        .expect_err("failed selection should not replace the active localizers");

    assert!(matches!(err, LocalizationError::LanguageNotSupported(_)));
    assert_eq!(
        manager.localize("selected-language", None),
        Some("en-US".to_string())
    );
}

#[test]
fn format_supported_languages_truncates_long_lists_for_diagnostics() {
    assert_eq!(
        format_supported_languages(DIAGNOSTIC_SUPPORTED_LANGUAGES),
        "en, fr, de, es, it, ja, +1 more"
    );
}

#[test]
fn format_module_support_includes_domain_when_it_differs() {
    assert_eq!(
        format_module_support(&DIAGNOSTIC_MODULE_DATA),
        "diagnostic-module (domain: diagnostic-domain, supports: en, fr, de, es, it, ja, +1 more)"
    );
}

#[test]
fn format_module_support_reports_missing_declared_languages() {
    assert_eq!(
        format_module_support(&MODULE_ERR_DATA),
        "module-err (supports: none declared)"
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
    assert_eq!(bundle.locales, vec![langid!("en-US"), langid!("en")]);

    let (localized, _format_errors) =
        localize_with_bundle(&bundle, "hello", None).expect("message should exist");
    assert_eq!(localized, "first");
}

#[test]
fn normalize_module_registry_skips_duplicate_name_and_domain() {
    let filtered = normalize_module_registry([
        &FILTER_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_DUP_NAME_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_DUP_DOMAIN_DESCRIPTOR as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].data().name, "filter-module");
}

#[test]
fn normalize_module_registry_prefers_runtime_localizer_for_exact_duplicate_identity() {
    let filtered = normalize_module_registry([
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_RUNTIME_MODULE as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].create_localizer().is_some());
}

#[test]
fn normalize_module_registry_keeps_metadata_only_registration_when_runtime_metadata_conflicts() {
    let filtered = normalize_module_registry([
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
        &FILTER_RUNTIME_MISMATCH_MODULE as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].create_localizer().is_none());
    assert_eq!(filtered[0].data(), &FILTER_EXACT_DUP_DATA);
}

#[test]
fn normalize_module_registry_keeps_runtime_localizer_when_metadata_duplicate_follows() {
    let filtered = normalize_module_registry([
        &FILTER_RUNTIME_MODULE as &dyn I18nModuleRegistration,
        &FILTER_EXACT_DUP_DESCRIPTOR as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].create_localizer().is_some());
}

#[test]
#[serial(explicit_runtime_create_calls)]
fn normalize_module_registry_uses_explicit_registration_kind_without_constructing_localizers() {
    EXPLICIT_RUNTIME_CREATE_CALLS.store(0, Ordering::Relaxed);

    let filtered =
        normalize_module_registry([&EXPLICIT_RUNTIME_REGISTRATION as &dyn I18nModuleRegistration]);

    assert_eq!(filtered.len(), 1);
    assert_eq!(EXPLICIT_RUNTIME_CREATE_CALLS.load(Ordering::Relaxed), 0);
}

#[test]
fn normalize_module_registry_skips_entries_with_invalid_metadata() {
    let filtered = normalize_module_registry([
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
