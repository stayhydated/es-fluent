#![doc = include_str!("../README.md")]

pub mod asset_localization;
pub mod embedded_localization;
pub mod fallback;
pub mod localization;
pub mod static_resource;

pub use asset_localization::{I18nModuleDescriptor, ModuleData, StaticModuleDescriptor};
pub use embedded_localization::{EmbeddedAssets, EmbeddedI18nModule};
pub use fallback::{fallback_locales, locale_candidates, resolve_fallback_language};
pub use localization::{
    FluentManager, I18nModule, LocalizationError, Localizer, build_fluent_args,
    localize_with_bundle,
};
pub use static_resource::StaticI18nResource;
