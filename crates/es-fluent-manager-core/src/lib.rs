#![doc = include_str!("../README.md")]

pub mod asset_localization;
pub mod embedded_localization;
pub mod fallback;
pub mod localization;

pub use asset_localization::{
    I18nModuleDescriptor, LocaleLoadReport, LocaleRelativeFtlPath, ModuleData, ModuleDomain,
    ModuleRegistryError, ModuleResourceSpec, ResourceKey, ResourceLoadError, ResourceLoadStatus,
    ResourcePlan, ResourcePlanError, StaticModuleDescriptor, load_locale_resource_entries,
    locale_is_ready, optional_resource_keys_from_plan, parse_fluent_resource_bytes,
    parse_fluent_resource_content, required_resource_keys_from_plan, resource_plan_for,
    try_resource_plan_for, validate_module_registry,
};
pub use embedded_localization::{EmbeddedI18nModule, EmbeddedLanguage, EmbeddedResource};
pub use es_fluent_shared::fluent::FluentDomain;
#[doc(hidden)]
pub use es_fluent_shared::registry::__macro;
pub use es_fluent_shared::registry::{
    StaticFluentArgumentName, StaticFluentDomain, StaticFluentEntryId, StaticFluentMessageKey,
    StaticFluentVariantKey,
};
pub use fallback::{
    FallbackChainAvailability, locale_candidates, resolve_fallback_chain_availability,
    resolve_fallback_language, resolve_ready_locale,
};
#[doc(hidden)]
pub use localization::log_unready_locale;
pub use localization::{
    BundleBuildError, DiscoveredRuntimeI18nModules, FluentArgumentMap, FluentManager, I18nModule,
    I18nModuleRegistration, LanguageSelectionPolicy, LocaleResources, LocalizationError, Localizer,
    ModuleDiscoveryError, ModuleLocalizer, ModuleRegistrationKind, SyncFluentBundle,
    add_resources_to_bundle, build_sync_bundle, localize_with_bundles, try_filter_module_registry,
};
