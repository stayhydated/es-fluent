use super::*;
use crate::asset_localization::{I18nModuleDescriptor, StaticModuleDescriptor};
use crate::localization::{I18nModule, LocalizationError};
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use unic_langid::{LanguageIdentifier, langid};

static MANAGER_INLINE_METADATA_DATA: ModuleData = ModuleData {
    name: "manager-inline-metadata",
    domain: "manager-inline-metadata",
    supported_languages: &[],
    namespaces: &[],
};
static MANAGER_INLINE_RUNTIME_DATA: ModuleData = ModuleData {
    name: "manager-inline-runtime",
    domain: "manager-inline-runtime",
    supported_languages: &[langid!("en")],
    namespaces: &[],
};
static MANAGER_INLINE_FOLLOWER_DATA: ModuleData = ModuleData {
    name: "manager-inline-follower",
    domain: "manager-inline-follower",
    supported_languages: &[langid!("en")],
    namespaces: &[],
};
static MANAGER_INLINE_METADATA: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&MANAGER_INLINE_METADATA_DATA);

struct ManagerInlineRuntimeModule;
struct ManagerInlineFollowerModule;
struct ManagerInlineLocalizer(&'static str);

impl Localizer for ManagerInlineLocalizer {
    fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (id == "inline").then(|| self.0.to_string())
    }
}

impl I18nModuleDescriptor for ManagerInlineRuntimeModule {
    fn data(&self) -> &'static ModuleData {
        &MANAGER_INLINE_RUNTIME_DATA
    }
}

impl I18nModule for ManagerInlineRuntimeModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(ManagerInlineLocalizer("runtime"))
    }
}

impl I18nModuleDescriptor for ManagerInlineFollowerModule {
    fn data(&self) -> &'static ModuleData {
        &MANAGER_INLINE_FOLLOWER_DATA
    }
}

impl I18nModule for ManagerInlineFollowerModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(ManagerInlineLocalizer("follower"))
    }

    fn contributes_to_language_selection(&self) -> bool {
        false
    }
}

static MANAGER_INLINE_RUNTIME: ManagerInlineRuntimeModule = ManagerInlineRuntimeModule;
static MANAGER_INLINE_FOLLOWER: ManagerInlineFollowerModule = ManagerInlineFollowerModule;

#[test]
fn load_runtime_modules_filters_metadata_only_registrations() {
    let modules = load_runtime_modules(vec![
        &MANAGER_INLINE_METADATA as &dyn I18nModuleRegistration,
        &MANAGER_INLINE_RUNTIME as &dyn I18nModuleRegistration,
    ]);

    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].data().name, "manager-inline-runtime");
}

#[test]
fn private_diagnostic_helpers_format_errors_and_missing_localizers() {
    let missing = unexpected_missing_localizer(&MANAGER_INLINE_RUNTIME_DATA);
    assert!(matches!(missing, LocalizationError::IoError(_)));
    assert!(missing.to_string().contains("manager-inline-runtime"));

    let formatted = format_module_discovery_errors(vec![
        ModuleDiscoveryError::InconsistentModuleMetadata {
            name: "one".to_string(),
            domain: "one-domain".to_string(),
        },
        ModuleDiscoveryError::DuplicateModuleRegistration {
            name: "two".to_string(),
            domain: "two-domain".to_string(),
            kind: ModuleRegistrationKind::MetadataOnly,
            count: 2,
        },
    ]);
    assert!(formatted.contains("- module 'one'"));
    assert!(formatted.contains("- module 'two'"));
}

#[test]
fn discovered_runtime_modules_support_empty_debug_clone_and_construction() {
    let discovered = DiscoveredRuntimeI18nModules {
        modules: Vec::<&'static dyn I18nModuleRegistration>::new().into(),
    };
    let cloned = discovered.clone();
    let manager = FluentManager::from_discovered_modules(&cloned);

    assert_eq!(discovered.len(), 0);
    assert!(discovered.is_empty());
    assert_eq!(
        format!("{discovered:?}"),
        "DiscoveredRuntimeI18nModules { len: 0 }"
    );
    assert!(manager.modules.is_empty());
}

#[test]
fn non_contributing_runtime_modules_follow_selection_without_declaring_support() {
    let manager = FluentManager {
        modules: vec![&MANAGER_INLINE_FOLLOWER as &dyn I18nModuleRegistration],
        localizers: RwLock::default(),
    };

    let err = manager
        .select_language(&langid!("en"))
        .expect_err("followers alone should not make a locale supported");

    assert!(matches!(err, LocalizationError::LanguageNotSupported(_)));
    assert_eq!(manager.localize("inline", None), None);
}

#[test]
fn runtime_modules_select_and_replace_active_localizers() {
    let manager = FluentManager {
        modules: vec![&MANAGER_INLINE_RUNTIME as &dyn I18nModuleRegistration],
        localizers: RwLock::default(),
    };

    manager
        .select_language_with_policy(&langid!("en"), LanguageSelectionPolicy::BestEffort)
        .expect("runtime module should support the locale");

    assert_eq!(
        manager.localize("inline", None),
        Some("runtime".to_string())
    );
    assert_eq!(
        manager.localize_in_domain("manager-inline-runtime", "inline", None),
        Some("runtime".to_string())
    );
}
