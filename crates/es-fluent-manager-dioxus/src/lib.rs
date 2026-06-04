#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use dioxus as __dioxus;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

pub use es_fluent_manager_macros::define_dioxus_i18n_module as define_i18n_module;

mod asset_loader;
mod error;

#[cfg(feature = "ssr")]
pub mod ssr;

pub use error::DioxusAssetI18nContextError;
pub use es_fluent_manager_core::LanguageSelectionPolicy;

pub use asset_loader::{
    DioxusAssetI18n, DioxusAssetLoadError, DioxusI18nAssetModule, DioxusI18nAssetModules,
    DioxusI18nAssetResource,
};

#[cfg(feature = "client")]
pub use asset_loader::{
    DioxusAssetI18nHandle, DioxusAssetI18nLoadState, DioxusAssetI18nProvider,
    DioxusAssetI18nReadyProvider, consume_asset_i18n, try_consume_asset_i18n, try_use_i18n,
    use_i18n, use_init_asset_i18n_modules, use_provide_asset_i18n,
};
