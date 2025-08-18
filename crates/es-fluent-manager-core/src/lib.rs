pub mod asset_localization;
pub mod localization;
pub mod static_localization;

pub use asset_localization::{AssetI18nModule, AssetModuleData, I18nAssetModule};
pub use localization::{FluentManager, I18nModule, LocalizationError, Localizer};
pub use static_localization::{StaticI18nModule, StaticLocalizer, StaticModuleData};
