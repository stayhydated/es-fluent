//! This module provides the core types for managing translations.

mod bundle;
mod manager;
mod module_localizer;
mod registry;

#[cfg(test)]
mod tests;

use crate::asset_localization::{
    I18nModuleDescriptor, ModuleResourceSpec, ResourceKey, StaticModuleDescriptor,
};
use es_fluent_shared::registry::StaticFluentMessageKey;
use unic_langid::LanguageIdentifier;

pub use bundle::{
    LocalizationError, SyncFluentBundle, add_resources_to_bundle, build_sync_bundle,
    localize_with_bundles,
};
pub use manager::{DiscoveredRuntimeI18nModules, FluentManager};
#[doc(hidden)]
pub use module_localizer::log_unready_locale;
pub use module_localizer::{BundleBuildError, LocaleResources, ModuleLocalizer};
pub use registry::{ModuleDiscoveryError, ModuleRegistrationKind, try_filter_module_registry};

pub type LocalizationErrorResult<T> = Result<T, LocalizationError>;
/// Fluent arguments passed to a localizer, keyed by argument name.
pub type FluentArgumentMap<'a> = fluent_bundle::FluentArgs<'a>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageSelectionPolicy {
    BestEffort,
    Strict,
}

/// Message lookup for one module in one selected locale.
///
/// Modules create a localizer per selected locale with
/// [`I18nModule::create_localizer`]; a localizer never changes locale.
pub trait Localizer: Send + Sync {
    /// Localizes a fully scoped static message key.
    fn localize<'a>(
        &self,
        key: StaticFluentMessageKey,
        args: Option<&FluentArgumentMap<'a>>,
    ) -> Option<String>;
}

/// Unified inventory contract for all module registrations.
///
/// Backends that only provide metadata (for example Bevy asset-driven loading)
/// can return `None` from `create_localizer`.
pub trait I18nModuleRegistration: I18nModuleDescriptor {
    /// Creates a localizer for `lang` when the registration supports runtime
    /// localization.
    ///
    /// Metadata-only registrations keep the default, which reports that the
    /// module has no runtime localizer.
    fn create_localizer(
        &self,
        _lang: &LanguageIdentifier,
    ) -> LocalizationErrorResult<Box<dyn Localizer>> {
        Err(std::io::Error::other(format!(
            "i18n module '{}' has no runtime localizer",
            self.data().name
        ))
        .into())
    }

    /// Returns the registration kind for duplicate-resolution and discovery.
    ///
    /// Manual registrations must implement this explicitly so discovery does
    /// not infer metadata by constructing a localizer.
    fn registration_kind(&self) -> ModuleRegistrationKind;

    /// Returns whether this registration can provide a runtime localizer.
    fn supports_runtime_localization(&self) -> bool {
        matches!(
            self.registration_kind(),
            ModuleRegistrationKind::RuntimeLocalizer
        )
    }

    /// Returns whether this registration should count as locale content support.
    ///
    /// Runtime utility modules can return `false` when they should follow the
    /// selected locale but must not make an otherwise unsupported locale look
    /// supported.
    fn contributes_to_language_selection(&self) -> bool {
        self.supports_runtime_localization()
    }

    /// Returns an optional manifest-derived resource plan for a specific language.
    ///
    /// This per-language plan is authoritative when present. It can be sparse:
    /// if a module has global namespaces `["ui", "errors"]` but a locale only
    /// ships `ui.ftl`, the returned plan should list only that locale's actual
    /// files. When this returns `Some`, managers should use it directly instead
    /// of expanding [`crate::asset_localization::ModuleData::resource_plan`] or
    /// probing for optional files.
    fn resource_plan_for_language(
        &self,
        _lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        None
    }

    /// Returns owner-provided Fluent source for a locale resource, when this
    /// registration owns loadable resource bytes.
    ///
    /// Asset-driven integrations may return `None` and let the host asset
    /// pipeline load the planned resource. Generated registrations that embed
    /// their FTL files can return `Some` to provide the bytes directly.
    fn resource_content_for_language(
        &self,
        _lang: &LanguageIdentifier,
        _resource_key: &ResourceKey,
    ) -> Option<&'static str> {
        None
    }
}

pub trait I18nModule: I18nModuleDescriptor {
    /// Creates a localizer for `lang`.
    ///
    /// Implementations own their fallback behavior. Generated localizers
    /// resolve parent locales internally; custom localizers that need the same
    /// behavior should resolve them here before returning
    /// [`LocalizationError::LanguageNotSupported`].
    fn create_localizer(
        &self,
        lang: &LanguageIdentifier,
    ) -> LocalizationErrorResult<Box<dyn Localizer>>;

    /// Returns an optional manifest-derived resource plan for a language.
    fn resource_plan_for_language(
        &self,
        _lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        None
    }

    /// Returns owner-provided Fluent source for a locale resource.
    fn resource_content_for_language(
        &self,
        _lang: &LanguageIdentifier,
        _resource_key: &ResourceKey,
    ) -> Option<&'static str> {
        None
    }

    /// Returns whether this module should count as locale content support.
    fn contributes_to_language_selection(&self) -> bool {
        true
    }
}

impl<T: I18nModule> I18nModuleRegistration for T {
    fn create_localizer(
        &self,
        lang: &LanguageIdentifier,
    ) -> LocalizationErrorResult<Box<dyn Localizer>> {
        I18nModule::create_localizer(self, lang)
    }

    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::RuntimeLocalizer
    }

    fn supports_runtime_localization(&self) -> bool {
        true
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        I18nModule::resource_plan_for_language(self, lang)
    }

    fn resource_content_for_language(
        &self,
        lang: &LanguageIdentifier,
        resource_key: &ResourceKey,
    ) -> Option<&'static str> {
        I18nModule::resource_content_for_language(self, lang, resource_key)
    }

    fn contributes_to_language_selection(&self) -> bool {
        I18nModule::contributes_to_language_selection(self)
    }
}

impl I18nModuleRegistration for StaticModuleDescriptor {
    fn registration_kind(&self) -> ModuleRegistrationKind {
        ModuleRegistrationKind::MetadataOnly
    }
}

inventory::collect!(&'static dyn I18nModuleRegistration);
