//! This module provides types for managing embedded translations.

mod assets;
mod bundle;
mod localizer;
mod module;

pub use assets::EmbeddedAssets;
pub use bundle::BundleBuildError;
pub use localizer::EmbeddedLocalizer;
pub use module::EmbeddedI18nModule;

#[cfg(test)]
use crate::{
    asset_localization::{ModuleData, ModuleDomain, ModuleResourceSpec},
    localization::{FluentArgumentMap, LocalizationError},
};
#[cfg(test)]
use assets::{embedded_resource_from_asset_path, parse_embedded_language_identifier};
#[cfg(test)]
use es_fluent_shared::registry::StaticFluentMessageKey;
#[cfg(test)]
use unic_langid::LanguageIdentifier;

#[cfg(test)]
mod tests;
