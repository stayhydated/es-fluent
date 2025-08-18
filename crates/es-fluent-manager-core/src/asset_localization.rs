use unic_langid::LanguageIdentifier;

/// Data for an asset-based i18n module that can be discovered via inventory.
/// This is used by asset-based managers to know what translations to load.
#[derive(Debug)]
pub struct AssetModuleData {
    /// The name of the module (typically the crate name)
    pub name: &'static str,
    /// The domain name used for asset file names (e.g., "my-app" -> "my-app.ftl")
    pub domain: &'static str,
    /// Languages supported by this module
    pub supported_languages: &'static [LanguageIdentifier],
}

/// Trait for asset-based i18n modules that can be discovered via inventory.
/// Unlike static modules that embed content at compile time, asset modules
/// provide metadata about what assets should be loaded at runtime.
pub trait I18nAssetModule: Send + Sync {
    /// Returns the module's metadata
    fn data(&self) -> &'static AssetModuleData;
}

/// Concrete implementation of an asset-based i18n module.
/// This is typically created by macros and registered via inventory.
pub struct AssetI18nModule {
    data: &'static AssetModuleData,
}

impl AssetI18nModule {
    /// Creates a new asset-based i18n module with the given data
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
