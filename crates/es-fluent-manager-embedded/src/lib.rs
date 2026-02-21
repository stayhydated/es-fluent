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
    let manager = FluentManager::new_with_discovered_modules();
    let manager_arc = Arc::new(manager);
    if GENERIC_MANAGER
        .set(ArcSwap::new(Arc::clone(&manager_arc)))
        .is_ok()
    {
        set_shared_context(manager_arc);
    } else {
        tracing::warn!("Generic fluent manager already initialized.");
    }
}

/// Selects the active language for the embedded singleton `FluentManager`.
///
/// After a language is selected, all subsequent calls to localization functions
/// will use the bundles for this language.
///
/// # Errors
///
/// This function will log an error if the embedded singleton has not been initialized by
/// calling `init()` first.
pub fn select_language<L: Into<LanguageIdentifier>>(lang: L) {
    if let Some(manager) = GENERIC_MANAGER.get() {
        manager.load().select_language(&lang.into());
    } else {
        tracing::error!("Generic fluent manager not initialized. Call init() first.");
    }
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use unic_langid::langid;

    static SELECT_CALLS: AtomicUsize = AtomicUsize::new(0);
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
    fn init_and_select_language_cover_singleton_paths() {
        // Exercise the pre-init error path.
        select_language(langid!("en-US"));
        assert!(GENERIC_MANAGER.get().is_none());

        init();
        assert!(GENERIC_MANAGER.get().is_some());

        select_language(langid!("en-US"));
        assert!(SELECT_CALLS.load(Ordering::Relaxed) >= 1);

        // Second init should hit the already-initialized branch.
        init();

        assert_eq!(es_fluent::localize("embedded-key", None), "embedded-value");
    }
}
