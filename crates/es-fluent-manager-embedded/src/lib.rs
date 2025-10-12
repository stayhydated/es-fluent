#![doc = include_str!("../README.md")]

use es_fluent::set_shared_context;
use es_fluent_manager_core::FluentManager;
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

static GENERIC_MANAGER: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();

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
    let manager_arc = Arc::new(RwLock::new(manager));
    if GENERIC_MANAGER.set(manager_arc.clone()).is_ok() {
        set_shared_context(manager_arc);
    } else {
        log::warn!("Generic fluent manager already initialized.");
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
pub fn select_language(lang: &LanguageIdentifier) {
    if let Some(manager_arc) = GENERIC_MANAGER.get() {
        let mut manager = manager_arc.write().unwrap();
        manager.select_language(lang);
    } else {
        log::error!("Generic fluent manager not initialized. Call init() first.");
    }
}
