//! This module provides the core types for managing translations.

mod bundle;
mod manager;
mod registry;

#[cfg(test)]
mod tests;

use crate::asset_localization::{I18nModuleDescriptor, ModuleResourceSpec, StaticModuleDescriptor};
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

pub use bundle::{
    LocalizationError, SyncFluentBundle, add_resources_to_bundle, build_fluent_args,
    build_sync_bundle, localize_with_bundle,
};
pub use manager::FluentManager;
pub use registry::filter_module_registry;

pub type LocalizationErrorResult<T> = Result<T, LocalizationError>;

pub trait Localizer: Send + Sync {
    /// Selects a language for the localizer.
    fn select_language(&self, lang: &LanguageIdentifier) -> es_fluent_shared::EsFluentResult<()>;
    /// Localizes a message by its ID.
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String>;
}

/// Unified inventory contract for all module registrations.
///
/// Backends that only provide metadata (for example Bevy asset-driven loading)
/// can return `None` from `create_localizer`.
pub trait I18nModuleRegistration: I18nModuleDescriptor {
    /// Creates a localizer when the registration supports runtime localization.
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        None
    }

    /// Returns whether this registration can provide a runtime localizer.
    ///
    /// Implementations can override this to avoid constructing a localizer just
    /// for capability checks during duplicate-resolution.
    fn supports_runtime_localization(&self) -> bool {
        self.create_localizer().is_some()
    }

    /// Returns an optional manifest-derived resource plan for a specific language.
    ///
    /// When this returns `Some`, managers should use this plan directly instead of
    /// inferring optional resource existence at runtime.
    fn resource_plan_for_language(
        &self,
        _lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        None
    }
}

pub trait I18nModule: I18nModuleDescriptor {
    /// Creates a localizer for the module.
    fn create_localizer(&self) -> Box<dyn Localizer>;
}

impl<T: I18nModule> I18nModuleRegistration for T {
    fn create_localizer(&self) -> Option<Box<dyn Localizer>> {
        Some(I18nModule::create_localizer(self))
    }

    fn supports_runtime_localization(&self) -> bool {
        true
    }
}

impl I18nModuleRegistration for StaticModuleDescriptor {}

inventory::collect!(&'static dyn I18nModuleRegistration);
