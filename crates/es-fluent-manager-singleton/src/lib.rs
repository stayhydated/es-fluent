use es_fluent::set_shared_context;
use es_fluent_manager_core::{FluentManager, I18nAssetModule};
use std::sync::{Arc, OnceLock, RwLock};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

static GENERIC_MANAGER: OnceLock<Arc<RwLock<FluentManager>>> = OnceLock::new();

pub fn init() {
    let manager = FluentManager::new_with_discovered_modules();
    let manager_arc = Arc::new(RwLock::new(manager));
    if GENERIC_MANAGER.set(manager_arc.clone()).is_ok() {
        set_shared_context(manager_arc);
    } else {
        log::warn!("Generic fluent manager already initialized.");
    }
}

pub fn select_language(lang: &LanguageIdentifier) {
    if let Some(manager_arc) = GENERIC_MANAGER.get() {
        let mut manager = manager_arc.write().unwrap();
        let _ = manager.select_language(lang);
    } else {
        log::error!("Generic fluent manager not initialized. Call init() first.");
    }
}

pub fn init_with_discovery() {
    let manager = FluentManager::new_with_discovered_modules();
    let manager_arc = Arc::new(RwLock::new(manager));

    log::info!("Generic fluent manager initialized with embedded and asset module discovery");
    log_discovered_asset_modules();

    if GENERIC_MANAGER.set(manager_arc.clone()).is_ok() {
        set_shared_context(manager_arc);
        log::info!("Generic fluent manager ready with discovered modules");
    } else {
        log::warn!("Generic fluent manager already initialized.");
    }
}

pub fn get_discovered_asset_modules() -> Vec<&'static dyn I18nAssetModule> {
    inventory::iter::<&'static dyn I18nAssetModule>()
        .map(|m| *m)
        .collect()
}

fn log_discovered_asset_modules() {
    let asset_modules = get_discovered_asset_modules();
    if !asset_modules.is_empty() {
        log::info!(
            "Discovered {} asset-based i18n modules:",
            asset_modules.len()
        );
        for module in asset_modules {
            let data = module.data();
            log::info!(
                "  - Module '{}' (domain: '{}') with {} languages: {:?}",
                data.name,
                data.domain,
                data.supported_languages.len(),
                data.supported_languages
            );
        }
        log::info!(
            "Note: Asset modules require a compatible manager (like es-fluent-manager-bevy) for runtime loading."
        );
        log::info!("Embedded modules are loaded automatically at compile time.");
    } else {
        log::debug!("No asset-based i18n modules discovered");
    }
}

pub fn get_asset_loading_info() -> Vec<(&'static str, &'static [LanguageIdentifier])> {
    get_discovered_asset_modules()
        .into_iter()
        .map(|module| {
            let data = module.data();
            (data.domain, data.supported_languages)
        })
        .collect()
}
