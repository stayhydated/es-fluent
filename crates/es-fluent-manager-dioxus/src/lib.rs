#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub use es_fluent::__inventory;

#[doc(hidden)]
pub use es_fluent::__rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

pub use es_fluent::GlobalLocalizationError;

mod bridge;
mod error;
mod managed;

#[cfg(feature = "client")]
mod client;

#[cfg(feature = "ssr")]
pub mod ssr;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_dioxus_i18n_module as define_i18n_module;

pub use bridge::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, GlobalBridgePolicy};
pub use error::{DioxusInitError, ModuleDiscoveryErrors};
pub use managed::ManagedI18n;

#[cfg(feature = "client")]
pub use client::{
    DioxusI18n, GlobalBridgeLocalizationExt, I18nProviderConfig, try_use_i18n,
    use_global_bridge_localized, use_i18n, use_i18n_provider_once, use_provide_i18n_once,
};

#[cfg(feature = "desktop")]
pub mod desktop {
    pub use crate::{
        DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusI18n, DioxusInitError,
        GlobalBridgeLocalizationExt, GlobalBridgePolicy, I18nProviderConfig, ManagedI18n,
        ModuleDiscoveryErrors, try_use_i18n, use_global_bridge_localized, use_i18n,
        use_i18n_provider_once, use_provide_i18n_once,
    };
}

#[cfg(feature = "mobile")]
pub mod mobile {
    pub use crate::{
        DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusI18n, DioxusInitError,
        GlobalBridgeLocalizationExt, GlobalBridgePolicy, I18nProviderConfig, ManagedI18n,
        ModuleDiscoveryErrors, try_use_i18n, use_global_bridge_localized, use_i18n,
        use_i18n_provider_once, use_provide_i18n_once,
    };
}

#[cfg(feature = "web")]
pub mod web {
    pub use crate::{
        DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusI18n, DioxusInitError,
        GlobalBridgeLocalizationExt, GlobalBridgePolicy, I18nProviderConfig, ManagedI18n,
        ModuleDiscoveryErrors, try_use_i18n, use_global_bridge_localized, use_i18n,
        use_i18n_provider_once, use_provide_i18n_once,
    };
}

#[cfg(test)]
mod tests;
