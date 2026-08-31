mod app;
mod assets;
mod discovery;
mod language;
#[cfg(feature = "file_watcher")]
mod watch;

pub(super) use app::{configure_app, register_discovered_fluent_text};
pub(super) use assets::{build_i18n_assets, register_discovered_i18n_assets};
pub(super) use discovery::discover_modules;
pub(super) use language::{initialize_i18n_resource, resolve_initial_language};

#[cfg(test)]
use crate::{
    BevyFluentTextRegistration, BevyI18nAssetRegistration, FtlAsset, I18nAssets, I18nResource,
    I18nSet, RequestedLanguageId,
};
#[cfg(test)]
use bevy::prelude::*;
#[cfg(test)]
use es_fluent_manager_core::{FluentManager, I18nModuleRegistration};
#[cfg(test)]
use language::initialize_i18n_resource_with_fallback_manager;
#[cfg(test)]
use unic_langid::LanguageIdentifier;
#[cfg(all(test, feature = "file_watcher"))]
use watch::{
    WatchedEmbeddedI18nAsset, WatchedEmbeddedI18nAssets, watch_embedded_i18n_asset_changes,
};

#[cfg(test)]
mod tests;
