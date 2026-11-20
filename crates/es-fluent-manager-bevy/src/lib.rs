#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub use bevy;

#[doc(hidden)]
pub use inventory;

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
mod schedule;
mod systems;

pub use components::FluentText;
pub use context::BevyI18n;
pub use locale::{
    ActiveLanguageId, FromLocale, LocaleChangeMessage, LocaleChangedMessage, RefreshForLocale,
    RequestedLanguageId,
};
pub use plugin::{I18nPlugin, I18nPluginConfig, I18nPluginStartupError};
pub use registration::FluentTextRegistration;
pub use schedule::I18nSet;

#[doc(hidden)]
pub use registration::BevyFluentTextRegistration;
#[doc(hidden)]
pub use registration::BevyI18nAssetRegistration;
#[doc(hidden)]
pub use registration::BevyI18nEmbeddedAsset;

/// Expands to the on-disk source path that `file_watcher` polls for a
/// package-relative i18n asset.
///
/// With `file_watcher`, the path comes from the registering package's
/// `CARGO_MANIFEST_DIR`, which rustc records as an input, so a compiler-cache
/// hit cannot point the watcher at another checkout. Otherwise the watcher is
/// absent and the package-relative path is kept, leaving the expansion
/// independent of the checkout location.
#[doc(hidden)]
#[cfg(feature = "file_watcher")]
#[macro_export]
macro_rules! __i18n_asset_source_path {
    ($relative:literal) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/", $relative)
    };
}

#[doc(hidden)]
#[cfg(not(feature = "file_watcher"))]
#[macro_export]
macro_rules! __i18n_asset_source_path {
    ($relative:literal) => {
        $relative
    };
}

pub(crate) use assets::{
    BundleBuildFailures, FluentResourceScope, FtlAsset, FtlAssetLoader, I18nAssets,
    I18nDomainBundles, I18nReadyLocales, I18nResource, I18nResourceKey,
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
