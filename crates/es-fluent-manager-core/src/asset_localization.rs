//! This module provides types for managing asset-based translations.

use unic_langid::LanguageIdentifier;

#[derive(Debug)]
pub struct AssetModuleData {
    /// The name of the module.
    pub name: &'static str,
    /// The domain of the module.
    pub domain: &'static str,
    /// The supported languages of the module.
    pub supported_languages: &'static [LanguageIdentifier],
    /// The namespaces used by this module's types (e.g., "ui", "errors").
    /// If empty, only the main domain file (e.g., `bevy-example.ftl`) is loaded.
    pub namespaces: &'static [&'static str],
}

pub trait I18nAssetModule: Send + Sync {
    /// Returns the data for the module.
    fn data(&self) -> &'static AssetModuleData;
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    static SUPPORTED: &[LanguageIdentifier] = &[langid!("en-US"), langid!("fr")];
    static NAMESPACES: &[&str] = &["ui", "errors"];
    static DATA: AssetModuleData = AssetModuleData {
        name: "test-module",
        domain: "test-domain",
        supported_languages: SUPPORTED,
        namespaces: NAMESPACES,
    };

    #[test]
    fn asset_module_new_and_data_round_trip() {
        let module = AssetI18nModule::new(&DATA);
        let data = module.data();

        assert_eq!(data.name, "test-module");
        assert_eq!(data.domain, "test-domain");
        assert_eq!(data.supported_languages, SUPPORTED);
        assert_eq!(data.namespaces, NAMESPACES);
    }
}
