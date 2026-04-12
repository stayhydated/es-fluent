use super::*;
use crate::asset_localization::{I18nModuleDescriptor, ModuleData, StaticModuleDescriptor};
use fluent_bundle::FluentResource;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use unic_langid::langid;

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
static FILTER_DESCRIPTOR: StaticModuleDescriptor = StaticModuleDescriptor::new(&FILTER_MODULE_DATA);
static FILTER_DUP_NAME_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_DUP_NAME_DATA);
static FILTER_DUP_DOMAIN_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_DUP_DOMAIN_DATA);
static FILTER_EXACT_DUP_DESCRIPTOR: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&FILTER_EXACT_DUP_DATA);

struct ModuleOk;
struct ModuleErr;
struct FilterRuntimeModule;

struct LocalizerOk;
struct LocalizerErr;
struct FilterRuntimeLocalizer;

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
        if id == "from-err" {
            Some("err-value".to_string())
        } else {
            None
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

static MODULE_OK: ModuleOk = ModuleOk;
static MODULE_ERR: ModuleErr = ModuleErr;
static FILTER_RUNTIME_MODULE: FilterRuntimeModule = FilterRuntimeModule;

inventory::submit! {
    &MODULE_OK as &dyn I18nModuleRegistration
}

inventory::submit! {
    &MODULE_ERR as &dyn I18nModuleRegistration
}

#[test]
fn manager_select_language_calls_all_localizers() {
    let ok_before = SELECT_OK_CALLS.load(Ordering::Relaxed);
    let err_before = SELECT_ERR_CALLS.load(Ordering::Relaxed);

    let manager = FluentManager::new_with_discovered_modules();
    manager.select_language(&langid!("en-US"));

    assert!(SELECT_OK_CALLS.load(Ordering::Relaxed) > ok_before);
    assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) > err_before);
}

#[test]
fn manager_localize_returns_first_matching_message() {
    let manager = FluentManager::new_with_discovered_modules();
    assert_eq!(
        manager.localize("from-ok", None),
        Some("ok-value".to_string())
    );
    assert_eq!(
        manager.localize("from-err", None),
        Some("err-value".to_string())
    );
    assert_eq!(manager.localize("missing", None), None);
}

#[test]
fn manager_select_language_with_only_failing_localizers_covers_warn_path() {
    let err_before = SELECT_ERR_CALLS.load(Ordering::Relaxed);

    let manager = FluentManager {
        localizers: vec![(&MODULE_ERR_DATA, Box::new(LocalizerErr))],
    };
    manager.select_language(&langid!("en-US"));

    assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) > err_before);
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
