#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub use bevy;

#[doc(hidden)]
pub use inventory;

#[doc(hidden)]
pub use es_fluent::__rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use inventory as __inventory;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::BevyFluentText;
#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_i18n_module;

#[doc(hidden)]
pub use unic_langid;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

mod assets;
mod components;
mod context;
mod locale;
mod plugin;
mod registration;
mod systems;

pub use components::FluentText;
pub use context::BevyI18n;
pub use locale::{
    ActiveLanguageId, FromLocale, LocaleChangeEvent, LocaleChangedEvent, RefreshForLocale,
    RequestedLanguageId,
};
pub use plugin::{I18nPlugin, I18nPluginConfig, I18nPluginStartupError};
pub use registration::FluentTextRegistration;

#[doc(hidden)]
pub use registration::BevyFluentTextRegistration;

pub(crate) use assets::{
    BundleBuildFailures, FtlAsset, FtlAssetLoader, I18nAssets, I18nBundle, I18nDomainBundles,
    I18nResource,
};
pub(crate) use es_fluent::FluentMessage;
#[cfg(test)]
pub(crate) use locale::primary_language;
pub(crate) use locale::{LanguageSelection, PendingLanguageChange, update_values_on_locale_change};
#[cfg(test)]
pub(crate) use registration::{EsFluentBevyPlugin, RegisteredFluentTextTypes};

#[doc(hidden)]
pub use unic_langid::langid as __langid;

#[cfg(test)]
mod tests;
