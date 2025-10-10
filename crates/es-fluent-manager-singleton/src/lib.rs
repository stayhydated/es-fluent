#![doc = include_str!("../README.md")]

use es_fluent::set_shared_context;
use es_fluent_manager_core::FluentManager;
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

static GENERIC_MANAGER: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();

/// Initializes the singleton.
///
/// This function should be called once at the beginning of your application's
/// lifecycle.
pub fn init() {
    let manager = FluentManager::new_with_discovered_modules();
    let manager_arc = Arc::new(RwLock::new(manager));
    if GENERIC_MANAGER.set(manager_arc.clone()).is_ok() {
        set_shared_context(manager_arc);
    } else {
        log::warn!("Generic fluent manager already initialized.");
    }
}

/// Selects a language for the singleton.
pub fn select_language(lang: &LanguageIdentifier) {
    if let Some(manager_arc) = GENERIC_MANAGER.get() {
        let mut manager = manager_arc.write().unwrap();
        manager.select_language(lang);
    } else {
        log::error!("Generic fluent manager not initialized. Call init() first.");
    }
}
