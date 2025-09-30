use unic_langid::LanguageIdentifier;

#[derive(Debug)]
pub struct AssetModuleData {
    pub name: &'static str,
    pub domain: &'static str,
    pub supported_languages: &'static [LanguageIdentifier],
}

pub trait I18nAssetModule: Send + Sync {
    fn data(&self) -> &'static AssetModuleData;
}

pub struct AssetI18nModule {
    data: &'static AssetModuleData,
}

impl AssetI18nModule {
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
