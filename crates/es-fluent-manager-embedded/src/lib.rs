#![doc = include_str!("../README.md")]

use es_fluent::{
    FluentArgs, FluentLocalizer, FluentLocalizerExt, FluentLocalizerLookup, FluentMessage,
    registry::StaticFluentMessageKey,
};
use es_fluent_manager_core::{FluentManager, ModuleDiscoveryError};
use std::sync::{Arc, RwLock};
use tracing::info;
use unic_langid::LanguageIdentifier;

#[doc(hidden)]
pub use es_fluent::__inventory;

#[doc(hidden)]
pub use rust_embed as __rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

pub use es_fluent_manager_core::LocalizationError;

#[derive(Debug)]
pub enum EmbeddedInitError {
    ModuleDiscovery(Vec<ModuleDiscoveryError>),
    LanguageSelection(LocalizationError),
}

impl std::fmt::Display for EmbeddedInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDiscovery(errors) => {
                f.write_str("failed strict i18n module discovery")?;
                for error in errors {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            },
            Self::LanguageSelection(error) => {
                write!(f, "failed to select the requested language: {error}")
            },
        }
    }
}

impl std::error::Error for EmbeddedInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection(error) => Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EmbeddedSelectionPolicy {
    BestEffort,
    Strict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveSelection {
    language: LanguageIdentifier,
    policy: EmbeddedSelectionPolicy,
}

/// Explicit embedded localization context.
///
/// Construct this once during application startup, keep it in application state,
/// and pass it to code that needs localization.
#[derive(Clone)]
pub struct EmbeddedI18n {
    manager: Arc<FluentManager>,
    active_selection: Arc<RwLock<Option<ActiveSelection>>>,
}

impl EmbeddedI18n {
    fn from_manager(manager: FluentManager) -> Self {
        Self::from_manager_with_active_selection(manager, None)
    }

    fn from_manager_with_active_selection(
        manager: FluentManager,
        active_selection: Option<ActiveSelection>,
    ) -> Self {
        Self {
            manager: Arc::new(manager),
            active_selection: Arc::new(RwLock::new(active_selection)),
        }
    }

    fn select_language_with_policy(
        &self,
        lang: LanguageIdentifier,
        policy: EmbeddedSelectionPolicy,
    ) -> Result<(), LocalizationError> {
        // Keep the cache check, manager transition, and cache commit in one
        // critical section shared by every clone and both selection policies.
        let mut active_selection = self
            .active_selection
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if active_selection
            .as_ref()
            .is_some_and(|selection| selection.language == lang && selection.policy == policy)
        {
            return Ok(());
        }

        info!("Changing locale to: {}", lang);
        match policy {
            EmbeddedSelectionPolicy::BestEffort => self.manager.select_language(&lang)?,
            EmbeddedSelectionPolicy::Strict => self.manager.select_language_strict(&lang)?,
        }
        #[cfg(test)]
        tests::after_manager_selection();
        *active_selection = Some(ActiveSelection {
            language: lang,
            policy,
        });
        Ok(())
    }

    /// Builds an embedded context without selecting a language.
    pub fn try_new() -> Result<Self, EmbeddedInitError> {
        FluentManager::try_new_with_discovered_modules()
            .map(Self::from_manager)
            .map_err(EmbeddedInitError::ModuleDiscovery)
    }

    /// Builds an embedded context and selects the initial active language.
    pub fn try_new_with_language<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, EmbeddedInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(EmbeddedInitError::ModuleDiscovery)?;
        manager
            .select_language(&lang)
            .map_err(EmbeddedInitError::LanguageSelection)?;
        Ok(Self::from_manager_with_active_selection(
            manager,
            Some(ActiveSelection {
                language: lang,
                policy: EmbeddedSelectionPolicy::BestEffort,
            }),
        ))
    }

    /// Builds an embedded context and selects the initial active language,
    /// failing if any runtime module rejects the requested locale.
    pub fn try_new_with_language_strict<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, EmbeddedInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(EmbeddedInitError::ModuleDiscovery)?;
        manager
            .select_language_strict(&lang)
            .map_err(EmbeddedInitError::LanguageSelection)?;
        Ok(Self::from_manager_with_active_selection(
            manager,
            Some(ActiveSelection {
                language: lang,
                policy: EmbeddedSelectionPolicy::Strict,
            }),
        ))
    }

    /// Selects the active language for this context.
    ///
    /// Concurrent selections through cloned handles are serialized. A failed
    /// selection preserves the previously active language and selection policy.
    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.select_language_with_policy(lang.into(), EmbeddedSelectionPolicy::BestEffort)
    }

    /// Selects the active language for this context and fails if any runtime
    /// module rejects the requested locale.
    ///
    /// Shares the same serialized transition as [`Self::select_language`].
    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.select_language_with_policy(lang.into(), EmbeddedSelectionPolicy::Strict)
    }

    /// Renders a derived typed message through this context.
    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message(self, message)
    }
}

impl FluentLocalizer for EmbeddedI18n {
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgs<'a>>,
    ) -> Option<String> {
        FluentManager::localize(self.manager.as_ref(), key, args.map(FluentArgs::as_raw))
    }

    fn with_lookup(&self, f: &mut dyn FnMut(&mut FluentLocalizerLookup<'_>)) {
        FluentManager::with_lookup(self.manager.as_ref(), &mut |lookup| {
            let mut typed_lookup = |key: StaticFluentMessageKey, args: Option<&FluentArgs<'_>>| {
                lookup(key, args.map(FluentArgs::as_raw))
            };
            f(&mut typed_lookup);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent_manager_core::{
        I18nModule, I18nModuleDescriptor, I18nModuleRegistration, Localizer, ModuleData,
    };
    use std::cell::RefCell;
    use std::sync::{Mutex, Once, mpsc};
    use std::time::Duration;
    use unic_langid::langid;

    thread_local! {
        static AFTER_MANAGER_SELECTION: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    }

    pub(super) fn after_manager_selection() {
        let hook = AFTER_MANAGER_SELECTION.with_borrow_mut(Option::take);
        if let Some(hook) = hook {
            hook();
        }
    }

    static TEST_SUPPORTED_LANGUAGES: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
    static TEST_MODULE_DATA: ModuleData = ModuleData {
        name: "embedded-test-module",
        owner: es_fluent_manager_core::__macro::static_domain("embedded-test-module"),
        supported_languages: TEST_SUPPORTED_LANGUAGES,
        domains: &[es_fluent_manager_core::ModuleDomain {
            domain: es_fluent_manager_core::__macro::static_domain("embedded-test-module"),
            namespaces: &[],
        }],
    };

    struct TestModule;

    struct TestLocalizer {
        selected: Mutex<LanguageIdentifier>,
    }

    impl I18nModuleDescriptor for TestModule {
        fn data(&self) -> &'static ModuleData {
            &TEST_MODULE_DATA
        }
    }

    impl I18nModule for TestModule {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(TestLocalizer {
                selected: Mutex::new(langid!("en-US")),
            })
        }
    }

    impl Localizer for TestLocalizer {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            if TEST_SUPPORTED_LANGUAGES
                .iter()
                .any(|candidate| candidate == lang)
            {
                let mut selected = self
                    .selected
                    .lock()
                    .expect("test localizer language lock should not be poisoned");
                *selected = lang.clone();
                Ok(())
            } else {
                Err(LocalizationError::LanguageNotSupported(lang.clone()))
            }
        }

        fn localize<'a>(
            &self,
            key: StaticFluentMessageKey,
            _args: Option<&es_fluent_manager_core::FluentArgumentMap<'a>>,
        ) -> Option<String> {
            if key.owner() != "embedded-test-module" || key.domain() != "embedded-test-module" {
                return None;
            }
            let selected = self
                .selected
                .lock()
                .expect("test localizer language lock should not be poisoned")
                .to_string();
            let value = match (selected.as_str(), key.id().as_str()) {
                ("en-US", "hello") => "Hello",
                ("fr", "hello") => "Bonjour",
                _ => return None,
            };

            Some(value.to_string())
        }
    }

    struct TestMessage;

    impl FluentMessage for TestMessage {
        fn to_fluent_string_with(
            &self,
            localize: &mut es_fluent::FluentMessageLookup<'_>,
        ) -> String {
            localize(
                es_fluent::registry::__macro::static_message_key(
                    "embedded-test-module",
                    es_fluent::registry::__macro::static_domain("embedded-test-module"),
                    es_fluent::registry::__macro::static_entry_id("hello"),
                ),
                None,
            )
        }
    }

    static TEST_MODULE: TestModule = TestModule;
    static INVENTORY_ONCE: Once = Once::new();

    crate::__inventory::submit!(&TEST_MODULE as &dyn I18nModuleRegistration);

    fn force_inventory_link() {
        INVENTORY_ONCE.call_once(|| {
            let _ = &TEST_MODULE;
        });
    }

    fn static_key(id: &'static str) -> StaticFluentMessageKey {
        es_fluent::registry::__macro::static_message_key(
            "embedded-test-module",
            es_fluent::registry::__macro::static_domain("embedded-test-module"),
            es_fluent::registry::__macro::static_entry_id(id),
        )
    }

    fn select_with_policy(
        i18n: &EmbeddedI18n,
        language: LanguageIdentifier,
        policy: EmbeddedSelectionPolicy,
    ) -> Result<(), LocalizationError> {
        match policy {
            EmbeddedSelectionPolicy::BestEffort => i18n.select_language(language),
            EmbeddedSelectionPolicy::Strict => i18n.select_language_strict(language),
        }
    }

    #[test]
    fn concurrent_selections_keep_cached_and_rendered_languages_consistent() {
        force_inventory_link();
        for first_policy in [
            EmbeddedSelectionPolicy::BestEffort,
            EmbeddedSelectionPolicy::Strict,
        ] {
            let second_policy = match first_policy {
                EmbeddedSelectionPolicy::BestEffort => EmbeddedSelectionPolicy::Strict,
                EmbeddedSelectionPolicy::Strict => EmbeddedSelectionPolicy::BestEffort,
            };
            let i18n = EmbeddedI18n::try_new().unwrap();
            select_with_policy(&i18n, langid!("en-US"), first_policy).unwrap();
            let (committed_tx, committed_rx) = mpsc::channel();
            let (resume_tx, resume_rx) = mpsc::channel();
            let (finished_tx, finished_rx) = mpsc::channel();
            let timeout = Duration::from_secs(10);

            std::thread::scope(|scope| {
                let first_i18n = i18n.clone();
                let first = scope.spawn(move || {
                    AFTER_MANAGER_SELECTION.set(Some(Box::new(move || {
                        committed_tx.send(()).unwrap();
                        resume_rx.recv_timeout(timeout).unwrap();
                    })));
                    select_with_policy(&first_i18n, langid!("fr"), first_policy)
                });
                committed_rx.recv_timeout(timeout).unwrap();

                // If the cache is unlocked after the manager commit, expose the old
                // race by completing the second selection before the first caches it.
                // Otherwise release the serialized first selection so the second runs.
                let selection_is_locked = i18n.active_selection.try_write().is_err();
                let second_i18n = i18n.clone();
                let second = scope.spawn(move || {
                    let result = select_with_policy(&second_i18n, langid!("en-US"), second_policy);
                    finished_tx.send(()).unwrap();
                    result
                });
                if selection_is_locked {
                    resume_tx.send(()).unwrap();
                    finished_rx.recv_timeout(timeout).unwrap();
                } else {
                    finished_rx.recv_timeout(timeout).unwrap();
                    resume_tx.send(()).unwrap();
                }
                first.join().unwrap().unwrap();
                second.join().unwrap().unwrap();
            });

            assert_eq!(i18n.localize_message(&TestMessage), "Hello");
            select_with_policy(&i18n, langid!("fr"), first_policy).unwrap();
            assert_eq!(i18n.localize_message(&TestMessage), "Bonjour");
        }
    }

    #[test]
    fn rejected_selections_preserve_the_active_language_and_policy() {
        force_inventory_link();
        let i18n = EmbeddedI18n::try_new_with_language(langid!("en-US")).unwrap();
        let initial = i18n.active_selection.read().unwrap().clone();
        for policy in [
            EmbeddedSelectionPolicy::BestEffort,
            EmbeddedSelectionPolicy::Strict,
        ] {
            std::assert_matches!(
                select_with_policy(&i18n, langid!("de"), policy),
                Err(LocalizationError::LanguageNotSupported(language)) if language == langid!("de")
            );
            assert_eq!(i18n.localize_message(&TestMessage), "Hello");
            assert_eq!(*i18n.active_selection.read().unwrap(), initial);
        }
    }

    #[test]
    fn embedded_i18n_instances_select_languages_independently() {
        force_inventory_link();
        let en = EmbeddedI18n::try_new_with_language(langid!("en-US"))
            .expect("en embedded i18n should initialize");
        let fr = EmbeddedI18n::try_new_with_language(langid!("fr"))
            .expect("fr embedded i18n should initialize");

        assert_eq!(
            es_fluent::FluentLocalizer::localize(&en, static_key("hello"), None),
            Some("Hello".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize(&fr, static_key("hello"), None),
            Some("Bonjour".to_string())
        );

        en.select_language(langid!("fr"))
            .expect("en manager should switch to fr");

        assert_eq!(
            es_fluent::FluentLocalizer::localize(&en, static_key("hello"), None),
            Some("Bonjour".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize(&fr, static_key("hello"), None),
            Some("Bonjour".to_string())
        );

        fr.select_language(langid!("en-US"))
            .expect("fr manager should switch to en-US");

        assert_eq!(
            es_fluent::FluentLocalizer::localize(&en, static_key("hello"), None),
            Some("Bonjour".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize(&fr, static_key("hello"), None),
            Some("Hello".to_string())
        );
    }

    #[test]
    fn embedded_i18n_facade_methods_delegate_to_manager_and_typed_lookup() {
        force_inventory_link();
        let i18n = EmbeddedI18n::try_new_with_language(langid!("en-US"))
            .expect("embedded i18n should initialize");

        assert_eq!(
            es_fluent::FluentLocalizer::localize(&i18n, static_key("hello"), None),
            Some("Hello".to_string())
        );
        assert_eq!(i18n.localize_message(&TestMessage), "Hello");
        assert!(
            i18n.select_language_strict(langid!("de")).is_err(),
            "strict selection should reject unsupported locales"
        );
    }

    #[test]
    fn embedded_i18n_strict_initialization_tracks_active_language() {
        force_inventory_link();
        let i18n = EmbeddedI18n::try_new_with_language_strict(langid!("en-US"))
            .expect("strict embedded i18n should initialize");

        i18n.select_language_strict(langid!("en-US"))
            .expect("strictly selecting the active language should be a no-op");
        assert_eq!(
            es_fluent::FluentLocalizer::localize(&i18n, static_key("hello"), None),
            Some("Hello".to_string())
        );
        assert_eq!(
            es_fluent::FluentLocalizer::localize(&i18n, static_key("unknown"), None),
            None
        );

        i18n.select_language(langid!("en-US"))
            .expect("best-effort selection should store its own active policy");
        i18n.select_language(langid!("en-US"))
            .expect("best-effort selecting the active language should be a no-op");
    }

    #[test]
    fn embedded_i18n_try_new_builds_context_before_language_selection() {
        force_inventory_link();
        let i18n = EmbeddedI18n::try_new().expect("embedded i18n should initialize");
        let cloned = i18n.clone();

        assert_eq!(
            es_fluent::FluentLocalizer::localize(&i18n, static_key("hello"), None),
            None
        );
        assert_eq!(i18n.try_localize_message(&TestMessage), None);
        cloned
            .select_language(langid!("fr"))
            .expect("language selection should work after initialization");
        assert_eq!(i18n.localize_message(&TestMessage), "Bonjour");
        assert_eq!(
            es_fluent::FluentLocalizer::localize(&i18n, static_key("hello"), None),
            Some("Bonjour".to_string())
        );
    }

    #[test]
    fn embedded_init_error_display_and_source_match_error_kind() {
        use es_fluent_manager_core::{ModuleDiscoveryError, ModuleRegistrationKind};
        use std::error::Error as _;

        let discovery = EmbeddedInitError::ModuleDiscovery(vec![
            ModuleDiscoveryError::DuplicateModuleRegistration {
                name: "app".to_string(),
                owner: "app".to_string(),
                kind: ModuleRegistrationKind::MetadataOnly,
                count: 2,
            },
        ]);
        assert!(
            discovery
                .to_string()
                .contains("failed strict i18n module discovery")
        );
        assert!(discovery.source().is_none());

        let selection = EmbeddedInitError::LanguageSelection(
            LocalizationError::LanguageNotSupported(langid!("de")),
        );
        assert!(
            selection
                .to_string()
                .contains("failed to select the requested language")
        );
        assert!(selection.source().is_some());
    }
}
