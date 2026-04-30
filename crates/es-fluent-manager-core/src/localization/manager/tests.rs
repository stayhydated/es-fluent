use super::*;
use crate::asset_localization::{I18nModuleDescriptor, StaticModuleDescriptor};
use crate::localization::{I18nModule, LocalizationError};
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
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
static MANAGER_SHARED_DOMAIN_FIRST_DATA: ModuleData = ModuleData {
    name: "manager-shared-domain-first",
    domain: "manager-shared-domain",
    supported_languages: &[langid!("en")],
    namespaces: &[],
};
static MANAGER_SHARED_DOMAIN_SECOND_DATA: ModuleData = ModuleData {
    name: "manager-shared-domain-second",
    domain: "manager-shared-domain",
    supported_languages: &[langid!("en")],
    namespaces: &[],
};
static MANAGER_SCOPED_LOOKUP_DATA: ModuleData = ModuleData {
    name: "manager-scoped-lookup",
    domain: "manager-scoped-lookup",
    supported_languages: &[langid!("en"), langid!("fr")],
    namespaces: &[],
};
static MANAGER_INLINE_METADATA: StaticModuleDescriptor =
    StaticModuleDescriptor::new(&MANAGER_INLINE_METADATA_DATA);

struct ManagerInlineRuntimeModule;
struct ManagerInlineFollowerModule;
struct ManagerInlineLocalizer(&'static str);
struct ManagerSharedDomainModule {
    data: &'static ModuleData,
    id: &'static str,
    value: &'static str,
}
struct ManagerSharedDomainLocalizer {
    id: &'static str,
    value: &'static str,
}
struct ManagerScopedLookupLocalizer {
    language: &'static str,
    child_seen: Option<Mutex<mpsc::Sender<()>>>,
    continue_child: Option<Mutex<mpsc::Receiver<()>>>,
}

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

impl Localizer for ManagerSharedDomainLocalizer {
    fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        (id == self.id).then(|| self.value.to_string())
    }
}

impl ManagerScopedLookupLocalizer {
    fn blocking(
        language: &'static str,
        child_seen: mpsc::Sender<()>,
        continue_child: mpsc::Receiver<()>,
    ) -> Self {
        Self {
            language,
            child_seen: Some(Mutex::new(child_seen)),
            continue_child: Some(Mutex::new(continue_child)),
        }
    }

    fn static_language(language: &'static str) -> Self {
        Self {
            language,
            child_seen: None,
            continue_child: None,
        }
    }
}

impl Localizer for ManagerScopedLookupLocalizer {
    fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
        Ok(())
    }

    fn localize<'a>(
        &self,
        id: &str,
        _args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        if id == "child" {
            if let Some(child_seen) = &self.child_seen {
                child_seen
                    .lock()
                    .expect("test child sender lock should not be poisoned")
                    .send(())
                    .expect("test should receive child lookup notification");
            }
            if let Some(continue_child) = &self.continue_child {
                continue_child
                    .lock()
                    .expect("test child receiver lock should not be poisoned")
                    .recv()
                    .expect("test should release child lookup");
            }
        }

        matches!(id, "child" | "parent").then(|| format!("{}-{id}", self.language))
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

impl I18nModuleDescriptor for ManagerSharedDomainModule {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl I18nModule for ManagerSharedDomainModule {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(ManagerSharedDomainLocalizer {
            id: self.id,
            value: self.value,
        })
    }
}

static MANAGER_INLINE_RUNTIME: ManagerInlineRuntimeModule = ManagerInlineRuntimeModule;
static MANAGER_INLINE_FOLLOWER: ManagerInlineFollowerModule = ManagerInlineFollowerModule;
static MANAGER_SHARED_DOMAIN_FIRST: ManagerSharedDomainModule = ManagerSharedDomainModule {
    data: &MANAGER_SHARED_DOMAIN_FIRST_DATA,
    id: "first-message",
    value: "first",
};
static MANAGER_SHARED_DOMAIN_SECOND: ManagerSharedDomainModule = ManagerSharedDomainModule {
    data: &MANAGER_SHARED_DOMAIN_SECOND_DATA,
    id: "second-message",
    value: "second",
};

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
fn non_contributing_runtime_modules_can_follow_externally_supported_locale() {
    let manager = FluentManager {
        modules: vec![&MANAGER_INLINE_FOLLOWER as &dyn I18nModuleRegistration],
        localizers: RwLock::default(),
    };

    manager
        .select_language_for_supported_locale(&langid!("en"))
        .expect("external support should let follower modules commit");

    assert_eq!(
        manager.localize("inline", None),
        Some("follower".to_string())
    );
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

#[test]
fn domain_scoped_lookup_searches_all_localizers_in_the_domain() {
    let manager = FluentManager {
        modules: vec![
            &MANAGER_SHARED_DOMAIN_FIRST as &dyn I18nModuleRegistration,
            &MANAGER_SHARED_DOMAIN_SECOND as &dyn I18nModuleRegistration,
        ],
        localizers: RwLock::default(),
    };

    manager
        .select_language_with_policy(&langid!("en"), LanguageSelectionPolicy::BestEffort)
        .expect("shared-domain modules should support the locale");

    assert_eq!(
        manager.localize_in_domain("manager-shared-domain", "first-message", None),
        Some("first".to_string())
    );
    assert_eq!(
        manager.localize_in_domain("manager-shared-domain", "second-message", None),
        Some("second".to_string())
    );
}

#[test]
fn with_lookup_holds_active_localizers_for_the_entire_callback() {
    let (child_seen_tx, child_seen_rx) = mpsc::channel();
    let (continue_child_tx, continue_child_rx) = mpsc::channel();
    let manager = Arc::new(FluentManager {
        modules: Vec::new(),
        localizers: RwLock::new(vec![(
            &MANAGER_SCOPED_LOOKUP_DATA,
            Box::new(ManagerScopedLookupLocalizer::blocking(
                "en",
                child_seen_tx,
                continue_child_rx,
            )) as Box<dyn Localizer>,
        )]),
    });

    let render_manager = Arc::clone(&manager);
    let render = std::thread::spawn(move || {
        let mut rendered = None;
        render_manager.with_lookup(&mut |lookup| {
            let child = lookup("manager-scoped-lookup", "child", None)
                .expect("child lookup should resolve");
            let parent = lookup("manager-scoped-lookup", "parent", None)
                .expect("parent lookup should resolve");
            rendered = Some(format!("{parent}:{child}"));
        });
        rendered.expect("with_lookup should run callback")
    });

    child_seen_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("render should reach the child lookup");

    let (swap_started_tx, swap_started_rx) = mpsc::channel();
    let (swap_done_tx, swap_done_rx) = mpsc::channel();
    let swap_manager = Arc::clone(&manager);
    let swap = std::thread::spawn(move || {
        swap_started_tx
            .send(())
            .expect("test should observe localizer swap start");
        *swap_manager.localizers.write() = vec![(
            &MANAGER_SCOPED_LOOKUP_DATA,
            Box::new(ManagerScopedLookupLocalizer::static_language("fr")) as Box<dyn Localizer>,
        )];
        swap_done_tx
            .send(())
            .expect("test should observe localizer swap completion");
    });

    swap_started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("localizer swap thread should start");
    assert!(
        swap_done_rx
            .recv_timeout(Duration::from_millis(50))
            .is_err(),
        "localizer swap completed while scoped lookup was still in progress"
    );

    continue_child_tx
        .send(())
        .expect("test should release the child lookup");
    assert_eq!(
        render
            .join()
            .expect("render thread should complete without panicking"),
        "en-parent:en-child"
    );
    swap_done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("localizer swap should complete after scoped lookup");
    swap.join()
        .expect("localizer swap thread should complete without panicking");

    assert_eq!(
        manager.localize_in_domain("manager-scoped-lookup", "parent", None),
        Some("fr-parent".to_string())
    );
}
