#![doc = include_str!("../README.md")]

pub mod asset_localization;
pub mod embedded_localization;
pub mod fallback;
pub mod localization;

pub use asset_localization::{
    I18nModuleDescriptor, LocaleLoadReport, ModuleData, ModuleRegistryError, ModuleResourceSpec,
    ResourceKey, ResourceLoadError, ResourceLoadStatus, StaticModuleDescriptor,
    build_locale_load_report, clear_locale_resource, collect_available_languages,
    collect_locale_resources, load_locale_resources, locale_is_ready,
    optional_resource_keys_from_plan, parse_and_store_locale_resource_content,
    parse_fluent_resource_bytes, parse_fluent_resource_content, record_failed_locale_resource,
    record_locale_resource_error, record_missing_locale_resource, required_resource_keys_from_plan,
    resource_plan_for, store_locale_resource, validate_module_registry,
};
pub use embedded_localization::{BundleBuildError, EmbeddedAssets, EmbeddedI18nModule};
pub use fallback::{
    FallbackChainAvailability, fallback_locales, locale_candidates,
    resolve_fallback_chain_availability, resolve_fallback_language, resolve_ready_locale,
};
pub use localization::{
    DiscoveredI18nModules, FluentManager, I18nModule, I18nModuleRegistration, LocalizationError,
    Localizer, ModuleDiscoveryError, ModuleRegistrationKind, SyncFluentBundle,
    add_resources_to_bundle, build_fluent_args, build_sync_bundle, fallback_errors_are_fatal,
    localize_with_bundle, localize_with_fallback_resources, try_filter_module_registry,
};
