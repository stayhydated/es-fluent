#![doc = include_str!("../README.md")]

pub mod asset_localization;
pub mod embedded_localization;
pub mod localization;

pub use asset_localization::{AssetI18nModule, AssetModuleData, I18nAssetModule};
pub use embedded_localization::{EmbeddedAssets, EmbeddedI18nModule, EmbeddedModuleData};
pub use localization::{FluentManager, I18nModule, LocalizationError, Localizer};
