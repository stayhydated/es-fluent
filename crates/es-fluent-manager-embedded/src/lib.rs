#![doc = include_str!("../README.md")]

#[doc(hidden)]
use arc_swap::ArcSwap;

#[doc(hidden)]
use es_fluent::try_set_shared_context;

#[doc(hidden)]
use es_fluent_manager_core::FluentManager;

#[doc(hidden)]
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock};

#[doc(hidden)]
use unic_langid::LanguageIdentifier;

pub use es_fluent::GlobalLocalizationError;

#[derive(Debug)]
pub enum EmbeddedInitError {
    ModuleDiscovery(Vec<es_fluent_manager_core::ModuleDiscoveryError>),
    GlobalContext(GlobalLocalizationError),
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
            Self::GlobalContext(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for EmbeddedInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::GlobalContext(error) => Some(error),
        }
    }
}

#[doc(hidden)]
pub use es_fluent::__inventory;

#[doc(hidden)]
pub use es_fluent::__rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

#[doc(hidden)]
static GENERIC_MANAGER: OnceLock<ArcSwap<FluentManager>> = OnceLock::new();

static INIT_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn init_lock() -> MutexGuard<'static, ()> {
    match INIT_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("Embedded manager init lock poisoned; recovering");
            poisoned.into_inner()
        },
    }
}

fn build_manager(
    initial_language: Option<&LanguageIdentifier>,
) -> Result<Arc<FluentManager>, EmbeddedInitError> {
    let manager = FluentManager::new_with_discovered_modules();
    if let Some(initial_language) = initial_language {
        select_initial_language(&manager, initial_language)?;
    }
    Ok(Arc::new(manager))
}

fn select_initial_language(
    manager: &FluentManager,
    initial_language: &LanguageIdentifier,
) -> Result<(), EmbeddedInitError> {
    manager
        .select_language(initial_language)
        .map_err(|error| EmbeddedInitError::GlobalContext(error.into()))
}

fn try_build_manager(
    initial_language: Option<&LanguageIdentifier>,
) -> Result<Arc<FluentManager>, EmbeddedInitError> {
    let manager = FluentManager::try_new_with_discovered_modules()
        .map_err(EmbeddedInitError::ModuleDiscovery)?;
    if let Some(initial_language) = initial_language {
        select_initial_language(&manager, initial_language)?;
    }
    Ok(Arc::new(manager))
}

fn initialize_manager(manager: Arc<FluentManager>) -> Result<bool, EmbeddedInitError> {
    let _guard = init_lock();

    if GENERIC_MANAGER.get().is_some() {
        tracing::warn!("Generic fluent manager already initialized.");
        return Ok(false);
    }

    try_set_shared_context(Arc::clone(&manager)).map_err(EmbeddedInitError::GlobalContext)?;

    if GENERIC_MANAGER.set(ArcSwap::new(manager)).is_ok() {
        Ok(true)
    } else {
        tracing::error!(
            "Embedded manager initialization published the shared context but failed to install the embedded singleton"
        );
        Err(EmbeddedInitError::GlobalContext(
            GlobalLocalizationError::ContextAlreadyInitialized,
        ))
    }
}

/// Initializes the embedded singleton `FluentManager`.
///
/// This function discovers all embedded i18n modules linked into the binary,
/// creates a `FluentManager` with them, and sets it as a global embedded singleton.
/// It also registers this manager with the `es-fluent` crate's central context,
/// allowing the `es_fluent::localize!` macro to work.
///
/// This function should be called once at the beginning of your application's
/// lifecycle.
///
pub fn init() {
    if let Err(error) = build_manager(None).and_then(initialize_manager) {
        tracing::error!("Failed to initialize embedded fluent manager: {}", error);
    }
}

/// Initializes the embedded singleton with strict registry discovery.
///
/// This is the opt-in startup path for applications that want duplicate or
/// invalid module registrations to fail immediately instead of being logged and
/// skipped.
pub fn try_init() -> Result<(), EmbeddedInitError> {
    if GENERIC_MANAGER.get().is_none() {
        let _ = initialize_manager(try_build_manager(None)?)?;
    }
    Ok(())
}

/// Initializes the embedded singleton `FluentManager` and selects the active language.
///
/// This is equivalent to calling [`init()`] followed by [`select_language()`], except the
/// language is selected before the manager is published as the global singleton.
/// If another thread initializes the singleton concurrently, `lang` is applied
/// to the live manager after the race is resolved.
///
pub fn init_with_language<L: Into<LanguageIdentifier>>(lang: L) {
    let lang = lang.into();
    if let Some(manager) = GENERIC_MANAGER.get() {
        tracing::warn!("Generic fluent manager already initialized.");
        if let Err(error) = manager.load().select_language(&lang) {
            tracing::error!(
                "Failed to apply language '{}' to the live embedded manager: {}",
                lang,
                error
            );
        }
        return;
    }

    match build_manager(Some(&lang)).and_then(initialize_manager) {
        Ok(true) => {},
        Ok(false) => {
            if let Some(manager) = GENERIC_MANAGER.get() {
                if let Err(error) = manager.load().select_language(&lang) {
                    tracing::error!(
                        "Failed to apply language '{}' after embedded init raced: {}",
                        lang,
                        error
                    );
                }
            } else {
                tracing::error!(
                    "Generic fluent manager initialization lost a race and no live manager was found."
                );
            }
        },
        Err(error) => {
            tracing::error!(
                "Failed to initialize embedded fluent manager with language '{}': {}",
                lang,
                error
            );
        },
    }
}

/// Initializes the embedded singleton with strict registry discovery and then
/// selects the active language.
///
/// If the requested language cannot be selected during initial construction,
/// this returns an error without publishing a global manager.
pub fn try_init_with_language<L: Into<LanguageIdentifier>>(
    lang: L,
) -> Result<(), EmbeddedInitError> {
    let lang = lang.into();
    if let Some(manager) = GENERIC_MANAGER.get() {
        return manager
            .load()
            .select_language(&lang)
            .map_err(|error| EmbeddedInitError::GlobalContext(error.into()));
    }

    let manager = try_build_manager(Some(&lang))?;
    if initialize_manager(manager)? {
        return Ok(());
    }

    if let Some(manager) = GENERIC_MANAGER.get() {
        manager
            .load()
            .select_language(&lang)
            .map_err(|error| EmbeddedInitError::GlobalContext(error.into()))
    } else {
        tracing::error!(
            "Generic fluent manager initialization lost a race and no live manager was found."
        );
        Err(EmbeddedInitError::GlobalContext(
            GlobalLocalizationError::ContextNotInitialized,
        ))
    }
}

/// Selects the active language for the embedded singleton `FluentManager`.
///
/// After a language is selected, all subsequent calls to localization functions
/// will use the bundles for this language.
///
/// # Errors
///
/// Returns an error if the embedded singleton has not been initialized by
/// calling `init()` first, or if no discovered module can serve the requested
/// language.
pub fn select_language<L: Into<LanguageIdentifier>>(
    lang: L,
) -> Result<(), GlobalLocalizationError> {
    let manager = GENERIC_MANAGER
        .get()
        .ok_or(GlobalLocalizationError::ContextNotInitialized)?;
    manager
        .load()
        .select_language(&lang.into())
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use es_fluent::FluentValue;
    use es_fluent_manager_core::{
        I18nModule, I18nModuleDescriptor, I18nModuleRegistration, LocalizationError, Localizer,
        ModuleData,
    };
    use std::collections::HashMap;
    use std::sync::{
        LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use unic_langid::langid;

    static SELECT_CALLS: AtomicUsize = AtomicUsize::new(0);
    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    static EMBEDDED_TEST_MODULE_DATA: ModuleData = ModuleData {
        name: "embedded-test-module",
        domain: "embedded-test-module",
        supported_languages: &[],
        namespaces: &[],
    };

    struct EmbeddedTestModule;
    struct EmbeddedTestLocalizer;

    impl Localizer for EmbeddedTestLocalizer {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            SELECT_CALLS.fetch_add(1, Ordering::Relaxed);
            if lang == &langid!("zz") {
                return Err(LocalizationError::LanguageNotSupported(lang.clone()));
            }
            Ok(())
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            if id == "embedded-key" {
                Some("embedded-value".to_string())
            } else {
                None
            }
        }
    }

    impl I18nModuleDescriptor for EmbeddedTestModule {
        fn data(&self) -> &'static ModuleData {
            &EMBEDDED_TEST_MODULE_DATA
        }
    }

    impl I18nModule for EmbeddedTestModule {
        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(EmbeddedTestLocalizer)
        }
    }

    static TEST_MODULE: EmbeddedTestModule = EmbeddedTestModule;

    crate::__inventory::submit! {
        &TEST_MODULE as &dyn I18nModuleRegistration
    }

    #[test]
    fn build_manager_selects_initial_language_when_requested() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");
        SELECT_CALLS.store(0, Ordering::Relaxed);

        let manager = build_manager(Some(&langid!("en-US")))
            .expect("lenient manager build should still apply the initial language");

        assert!(SELECT_CALLS.load(Ordering::Relaxed) >= 1);
        assert_eq!(
            manager.localize("embedded-key", None),
            Some("embedded-value".to_string())
        );
    }

    #[test]
    fn build_manager_rejects_unselectable_initial_language() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");
        SELECT_CALLS.store(0, Ordering::Relaxed);

        let err = match build_manager(Some(&langid!("zz"))) {
            Ok(_) => panic!("initial language selection failure should abort publication"),
            Err(err) => err,
        };

        assert!(SELECT_CALLS.load(Ordering::Relaxed) >= 1);
        assert!(matches!(
            err,
            EmbeddedInitError::GlobalContext(GlobalLocalizationError::LanguageSelectionFailed(_))
        ));
    }

    #[test]
    fn init_and_select_language_cover_singleton_paths() {
        let _guard = TEST_LOCK.lock().expect("lock poisoned");
        SELECT_CALLS.store(0, Ordering::Relaxed);

        // Exercise the pre-init error path.
        let err = select_language(langid!("en-US")).expect_err("selecting before init should fail");
        assert!(matches!(
            err,
            GlobalLocalizationError::ContextNotInitialized
        ));
        assert!(GENERIC_MANAGER.get().is_none());

        let strict_init_err = try_init_with_language(langid!("zz"))
            .expect_err("strict init should fail before publishing when selection fails");
        assert!(matches!(
            strict_init_err,
            EmbeddedInitError::GlobalContext(GlobalLocalizationError::LanguageSelectionFailed(_))
        ));
        assert!(GENERIC_MANAGER.get().is_none());

        SELECT_CALLS.store(0, Ordering::Relaxed);

        try_init_with_language(langid!("en-US"))
            .expect("strict init with language should validate, select once, and initialize");
        assert!(GENERIC_MANAGER.get().is_some());
        assert_eq!(SELECT_CALLS.load(Ordering::Relaxed), 1);

        select_language(langid!("en-US")).expect("initialized manager should select language");
        let after_explicit_select = SELECT_CALLS.load(Ordering::Relaxed);
        assert_eq!(after_explicit_select, 2);

        try_init().expect("strict init should be a no-op once initialized");

        // Re-initialization with a language should still apply the requested selection.
        let after_try_init_with_language = SELECT_CALLS.load(Ordering::Relaxed);
        try_init_with_language(langid!("de"))
            .expect("strict init with language should apply selection on the live manager");
        assert_eq!(
            SELECT_CALLS.load(Ordering::Relaxed),
            after_try_init_with_language + 1
        );

        let after_second_try_init_with_language = SELECT_CALLS.load(Ordering::Relaxed);
        init_with_language(langid!("fr"));
        assert_eq!(
            SELECT_CALLS.load(Ordering::Relaxed),
            after_second_try_init_with_language + 1
        );

        // Plain init should still hit the already-initialized branch without changing language.
        let after_reinit_with_language = SELECT_CALLS.load(Ordering::Relaxed);
        init();
        assert_eq!(
            SELECT_CALLS.load(Ordering::Relaxed),
            after_reinit_with_language
        );

        assert_eq!(es_fluent::localize("embedded-key", None), "embedded-value");
    }
}
