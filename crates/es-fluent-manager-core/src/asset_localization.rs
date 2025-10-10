//! This module provides types for managing asset-based translations.

use unic_langid::LanguageIdentifier;

/// Data for an asset-based i18n module.
#[derive(Debug)]
pub struct AssetModuleData {
    /// The name of the module.
    pub name: &'static str,
    /// The domain of the module.
    pub domain: &'static str,
    /// The supported languages of the module.
    pub supported_languages: &'static [LanguageIdentifier],
}

/// A trait for an asset-based i18n module.
pub trait I18nAssetModule: Send + Sync {
    /// Returns the data for the module.
    fn data(&self) -> &'static AssetModuleData;
}

/// An asset-based i18n module.
pub struct AssetI18nModule {
    data: &'static AssetModuleData,
}

impl AssetI18nModule {
    /// Creates a new `AssetI18nModule`.
    pub const fn new(data: &'static AssetModuleData) -> Self {
        Self { data }
    }
}

impl I18nAssetModule for AssetI18nModule {
    fn data(&self) -> &'static AssetModuleData {
        self.data
    }
}

inventory::collect!(&'static dyn I18nAssetModule);
