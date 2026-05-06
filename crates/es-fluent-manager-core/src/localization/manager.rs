use super::{
    I18nModuleRegistration, LanguageSelectionPolicy, Localizer, ModuleDiscoveryError,
    ModuleRegistrationKind,
};
use crate::asset_localization::ModuleData;
use fluent_bundle::FluentValue;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

type ManagedLocalizer = (&'static ModuleData, Box<dyn Localizer>);
const MAX_DIAGNOSTIC_LANGUAGES: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LanguageSupportRequirement {
    ContributingModule,
    RuntimeLocalizer,
}

/// Cached, validated runtime-capable i18n module registrations.
///
/// This lets integrations pay strict inventory discovery once, then construct
/// request-local [`FluentManager`] values cheaply from the cached module list.
#[derive(Clone)]
pub struct DiscoveredRuntimeI18nModules {
    modules: Arc<[&'static dyn I18nModuleRegistration]>,
}

impl DiscoveredRuntimeI18nModules {
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl std::fmt::Debug for DiscoveredRuntimeI18nModules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveredRuntimeI18nModules")
            .field("len", &self.len())
            .finish()
    }
}

/// A manager for Fluent translations.
pub struct FluentManager {
    pub(super) modules: Vec<&'static dyn I18nModuleRegistration>,
    pub(super) localizers: RwLock<Vec<ManagedLocalizer>>,
}

fn load_runtime_modules(
    discovered_modules: Vec<&'static dyn I18nModuleRegistration>,
) -> Vec<&'static dyn I18nModuleRegistration> {
    let mut modules = Vec::new();

    for module in discovered_modules {
        let data = module.data();
        tracing::info!("Discovered and loading i18n module: {}", data.name);
        match module.registration_kind() {
            ModuleRegistrationKind::RuntimeLocalizer => {
                modules.push(module);
            },
            ModuleRegistrationKind::MetadataOnly => {
                tracing::debug!(
                    "Skipping metadata-only i18n module '{}' for FluentManager runtime localization",
                    data.name
                );
            },
        }
    }

    modules
}

fn unexpected_missing_localizer(module: &ModuleData) -> crate::localization::LocalizationError {
    io::Error::other(format!(
        "runtime i18n module '{}' did not create a localizer during language selection",
        module.name
    ))
    .into()
}

fn format_module_discovery_errors(errors: Vec<ModuleDiscoveryError>) -> String {
    errors
        .into_iter()
        .map(|error| format!("- {error}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn format_module_names(modules: &[&'static ModuleData]) -> String {
    if modules.is_empty() {
        return "<none>".to_string();
    }

    modules
        .iter()
        .map(|data| data.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_supported_languages(languages: &[LanguageIdentifier]) -> String {
    if languages.is_empty() {
        return "none declared".to_string();
    }

    let mut formatted = languages
        .iter()
        .take(MAX_DIAGNOSTIC_LANGUAGES)
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if languages.len() > MAX_DIAGNOSTIC_LANGUAGES {
        formatted.push(format!(
            "+{} more",
            languages.len() - MAX_DIAGNOSTIC_LANGUAGES
        ));
    }

    formatted.join(", ")
}

pub(crate) fn format_module_support(data: &ModuleData) -> String {
    if data.domain == data.name {
        return format!(
            "{} (supports: {})",
            data.name,
            format_supported_languages(data.supported_languages)
        );
    }

    format!(
        "{} (domain: {}, supports: {})",
        data.name,
        data.domain,
        format_supported_languages(data.supported_languages)
    )
}

pub(crate) fn format_module_support_list(modules: &[&'static ModuleData]) -> String {
    if modules.is_empty() {
        return "<none>".to_string();
    }

    modules
        .iter()
        .map(|data| format_module_support(data))
        .collect::<Vec<_>>()
        .join(", ")
}

impl FluentManager {
    /// Creates a new `FluentManager` with strict discovered-module validation.
    pub fn new_with_discovered_modules() -> Self {
        Self::try_new_with_discovered_modules().unwrap_or_else(|errors| {
            panic!(
                "failed strict i18n module discovery:\n{}",
                format_module_discovery_errors(errors)
            )
        })
    }

    /// Creates a new `FluentManager` with strict registry validation.
    ///
    /// This returns an error instead of panicking when discovery finds invalid
    /// module metadata or unresolvable duplicate registrations.
    pub fn try_new_with_discovered_modules() -> Result<Self, Vec<ModuleDiscoveryError>> {
        Self::try_discover_runtime_modules().map(|modules| Self::from_discovered_modules(&modules))
    }

    /// Discovers, validates, and caches runtime-capable i18n modules.
    pub fn try_discover_runtime_modules()
    -> Result<DiscoveredRuntimeI18nModules, Vec<ModuleDiscoveryError>> {
        let discovered_modules = super::try_filter_module_registry(
            inventory::iter::<&'static dyn I18nModuleRegistration>()
                .copied()
                .collect::<Vec<_>>(),
        )?;

        Ok(DiscoveredRuntimeI18nModules {
            modules: load_runtime_modules(discovered_modules).into(),
        })
    }

    /// Creates a new manager from previously discovered runtime modules.
    pub fn from_discovered_modules(discovered: &DiscoveredRuntimeI18nModules) -> Self {
        Self {
            modules: discovered.modules.iter().copied().collect(),
            localizers: RwLock::default(),
        }
    }

    /// Selects a language for all localizers.
    ///
    /// By default this is best-effort: modules that report
    /// `LanguageNotSupported` are skipped as long as at least one module
    /// accepts the requested locale.
    pub fn select_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> crate::localization::LocalizationErrorResult<()> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::BestEffort)
    }

    /// Selects a language for all localizers and fails if any module rejects it.
    pub fn select_language_strict(
        &self,
        lang: &LanguageIdentifier,
    ) -> crate::localization::LocalizationErrorResult<()> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::Strict)
    }

    /// Selects runtime localizers after another backend has already confirmed
    /// application content support for the locale.
    ///
    /// This is intended for integrations such as Bevy, where asset-backed
    /// metadata proves locale support and runtime utility modules only need to
    /// follow the already accepted locale.
    pub fn select_language_for_supported_locale(
        &self,
        lang: &LanguageIdentifier,
    ) -> crate::localization::LocalizationErrorResult<()> {
        self.select_language_with_support_requirement(
            lang,
            LanguageSelectionPolicy::BestEffort,
            LanguageSupportRequirement::RuntimeLocalizer,
        )
    }

    /// Selects a language for all localizers using the requested policy.
    pub fn select_language_with_policy(
        &self,
        lang: &LanguageIdentifier,
        policy: LanguageSelectionPolicy,
    ) -> crate::localization::LocalizationErrorResult<()> {
        self.select_language_with_support_requirement(
            lang,
            policy,
            LanguageSupportRequirement::ContributingModule,
        )
    }

    fn select_language_with_support_requirement(
        &self,
        lang: &LanguageIdentifier,
        policy: LanguageSelectionPolicy,
        support_requirement: LanguageSupportRequirement,
    ) -> crate::localization::LocalizationErrorResult<()> {
        let mut next_localizers = Vec::with_capacity(self.modules.len());
        let mut selected_modules = Vec::with_capacity(self.modules.len());
        let checked_modules = self
            .modules
            .iter()
            .map(|module| module.data())
            .collect::<Vec<_>>();
        let mut any_contributing_selected = false;
        let mut any_runtime_selected = false;
        let mut first_failure = None;
        let mut first_non_unsupported_failure = None;
        let mut unsupported_modules = Vec::new();

        for module in &self.modules {
            let data = module.data();
            let Some(localizer) = module.create_localizer() else {
                let error = unexpected_missing_localizer(data);
                tracing::error!(
                    "Module '{}' failed to create a runtime localizer for '{}': {}",
                    data.name,
                    lang,
                    error
                );
                if first_non_unsupported_failure.is_none() {
                    first_non_unsupported_failure = Some((data, error));
                }
                continue;
            };

            match localizer.select_language(lang) {
                Ok(()) => {
                    any_runtime_selected = true;
                    selected_modules.push(data);
                    if module.contributes_to_language_selection() {
                        any_contributing_selected = true;
                    } else {
                        tracing::trace!(
                            "Activated non-selecting i18n module '{}' for language '{}'",
                            data.name,
                            lang
                        );
                    }
                    next_localizers.push((data, localizer));
                },
                Err(error) => {
                    tracing::debug!(
                        "Module '{}' failed to set language '{}': {}",
                        data.name,
                        lang,
                        error
                    );
                    if matches!(
                        &error,
                        crate::localization::LocalizationError::LanguageNotSupported(_)
                    ) {
                        unsupported_modules.push(data);
                        if first_failure.is_none() {
                            first_failure = Some(error);
                        }
                    } else if first_non_unsupported_failure.is_none() {
                        first_non_unsupported_failure = Some((data, error));
                    }
                },
            }
        }

        if let Some((module, error)) = first_non_unsupported_failure {
            tracing::warn!(
                "Language selection for '{}' failed because module '{}' returned a runtime-localizer error: {}; keeping the previous language active",
                lang,
                module.name,
                error,
            );
            return Err(error);
        }

        let has_required_support = match support_requirement {
            LanguageSupportRequirement::ContributingModule => any_contributing_selected,
            LanguageSupportRequirement::RuntimeLocalizer => any_runtime_selected,
        };

        if has_required_support
            && policy == LanguageSelectionPolicy::Strict
            && let Some(error) = first_failure
        {
            tracing::warn!(
                "Language selection for '{}' failed in strict mode; modules that accepted it: {}; modules that rejected it: {}; keeping the previous language active",
                lang,
                format_module_names(&selected_modules),
                format_module_support_list(&unsupported_modules),
            );
            return Err(error);
        }

        if !has_required_support {
            match support_requirement {
                LanguageSupportRequirement::ContributingModule => {
                    tracing::warn!(
                        "No i18n modules support language '{}'; modules checked: {}",
                        lang,
                        format_module_support_list(&checked_modules)
                    );
                },
                LanguageSupportRequirement::RuntimeLocalizer => {
                    tracing::warn!(
                        "No runtime i18n modules accepted externally supported language '{}'; modules checked: {}",
                        lang,
                        format_module_support_list(&checked_modules)
                    );
                },
            }
            return Err(crate::localization::LocalizationError::LanguageNotSupported(lang.clone()));
        }

        if !unsupported_modules.is_empty() {
            tracing::warn!(
                "Language '{}' is only partially supported; active modules: {}; skipped unsupported modules: {}",
                lang,
                format_module_names(&selected_modules),
                format_module_support_list(&unsupported_modules),
            );
        }

        *self.localizers.write() = next_localizers;
        Ok(())
    }

    /// Localizes a message by its ID.
    ///
    /// This searches localizers in discovery order and returns the first match.
    /// Use [`Self::localize_in_domain`] when the caller needs domain-scoped
    /// lookup instead of first-match behavior.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (_, localizer) in self.localizers.read().iter() {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }

    /// Localizes a message by its ID within a specific domain.
    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (data, localizer) in self.localizers.read().iter() {
            if data.domain == domain
                && let Some(message) = localizer.localize(id, args)
            {
                return Some(message);
            }
        }

        None
    }

    /// Runs a group of domain-scoped lookups against the current localizer set.
    ///
    /// The active localizer list is read-locked for the entire callback so
    /// nested typed-message lookups cannot observe a partially switched locale.
    pub fn with_lookup(
        &self,
        f: &mut dyn FnMut(
            &mut dyn for<'a> FnMut(
                &str,
                &str,
                Option<&HashMap<&str, FluentValue<'a>>>,
            ) -> Option<String>,
        ),
    ) {
        let localizers = self.localizers.read();
        let mut lookup = |domain: &str, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            for (data, localizer) in localizers.iter() {
                if data.domain == domain
                    && let Some(message) = localizer.localize(id, args)
                {
                    return Some(message);
                }
            }

            None
        };
        f(&mut lookup);
    }
}

#[cfg(test)]
mod tests {
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
}
