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
pub use es_fluent_manager_macros::define_dioxus_i18n_module as define_i18n_module;

mod bridge;
mod error;
mod managed;

#[cfg(feature = "client")]
pub mod client;

#[cfg(feature = "ssr")]
pub mod ssr;

pub use bridge::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner};
pub use error::{DioxusInitError, ModuleDiscoveryErrors};
pub use managed::ManagedI18n;

#[cfg(feature = "client")]
pub use client::{
    DioxusClientBridgeMode, DioxusI18n, I18nProvider, consume_i18n, try_consume_i18n, try_use_i18n,
    try_use_i18n_subscription, use_i18n, use_i18n_optional, use_i18n_subscription, use_init_i18n,
    use_init_i18n_with_bridge_mode, use_provide_i18n, use_provide_i18n_with_bridge_mode,
};

#[cfg(test)]
mod tests;
