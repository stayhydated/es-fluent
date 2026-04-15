#![doc = include_str!("../README.md")]

#[doc(hidden)]
use arc_swap::ArcSwap;

#[doc(hidden)]
use es_fluent::set_shared_context;

#[doc(hidden)]
use es_fluent_manager_core::FluentManager;

#[doc(hidden)]
use std::sync::{Arc, OnceLock};

#[doc(hidden)]
use unic_langid::LanguageIdentifier;

pub use es_fluent::GlobalLocalizationError;

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

fn build_manager(initial_language: Option<&LanguageIdentifier>) -> Arc<FluentManager> {
    let manager = FluentManager::new_with_discovered_modules();
    if let Some(initial_language) = initial_language {
        let _ = manager.select_language(initial_language);
    }
    Arc::new(manager)
}

fn initialize_manager(manager: Arc<FluentManager>) -> bool {
    if GENERIC_MANAGER
        .set(ArcSwap::new(Arc::clone(&manager)))
        .is_ok()
    {
        set_shared_context(manager);
        true
    } else {
        tracing::warn!("Generic fluent manager already initialized.");
        false
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
/// # Panics
///
/// This function will not panic if called more than once, but it will log a
/// warning and have no effect after the first successful call.
pub fn init() {
    let _ = initialize_manager(build_manager(None));
}

/// Initializes the embedded singleton `FluentManager` and selects the active language.
///
/// This is equivalent to calling [`init()`] followed by [`select_language()`], except the
/// language is selected before the manager is published as the global singleton.
/// If another thread initializes the singleton concurrently, `lang` is applied
/// to the live manager after the race is resolved.
///
/// # Panics
///
/// This function will not panic if called more than once. If the singleton is
/// already initialized, it logs a warning and applies `lang` to the existing
/// manager.
pub fn init_with_language<L: Into<LanguageIdentifier>>(lang: L) {
    let lang = lang.into();
    if let Some(manager) = GENERIC_MANAGER.get() {
        tracing::warn!("Generic fluent manager already initialized.");
        let _ = manager.load().select_language(&lang);
        return;
    }

    if !initialize_manager(build_manager(Some(&lang))) {
        if let Some(manager) = GENERIC_MANAGER.get() {
            let _ = manager.load().select_language(&lang);
        } else {
            tracing::error!(
                "Generic fluent manager initialization lost a race and no live manager was found."
            );
        }
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
        fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            SELECT_CALLS.fetch_add(1, Ordering::Relaxed);
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

        let manager = build_manager(Some(&langid!("en-US")));

        assert!(SELECT_CALLS.load(Ordering::Relaxed) >= 1);
        assert_eq!(
            manager.localize("embedded-key", None),
            Some("embedded-value".to_string())
        );
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

        init();
        assert!(GENERIC_MANAGER.get().is_some());

        select_language(langid!("en-US")).expect("initialized manager should select language");
        let after_explicit_select = SELECT_CALLS.load(Ordering::Relaxed);
        assert!(after_explicit_select >= 1);

        // Re-initialization with a language should still apply the requested selection.
        init_with_language(langid!("fr"));
        assert!(SELECT_CALLS.load(Ordering::Relaxed) > after_explicit_select);

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
