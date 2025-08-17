pub mod localization;
pub mod static_localization;

pub use localization::{FluentManager, I18nModule, LocalizationError, Localizer};
pub use static_localization::{StaticI18nModule, StaticLocalizer, StaticModuleData};