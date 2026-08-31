use crate::{
    asset_localization::{I18nModuleDescriptor, ModuleData, ModuleResourceSpec},
    localization::{I18nModule, Localizer},
};
use std::collections::HashSet;
use unic_langid::LanguageIdentifier;

use super::{EmbeddedAssets, EmbeddedLocalizer, assets::embedded_resource_from_asset_path};

pub struct EmbeddedI18nModule<T: EmbeddedAssets> {
    data: &'static ModuleData,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: EmbeddedAssets> EmbeddedI18nModule<T> {
    pub const fn new(data: &'static ModuleData) -> Self {
        Self {
            data,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn discover_languages() -> Vec<LanguageIdentifier> {
        let mut languages = Vec::new();
        let mut seen = HashSet::new();

        for file_path in T::iter() {
            let file_path_str = file_path.as_ref();
            for domain in T::domains() {
                if let Some((lang_id, _)) = embedded_resource_from_asset_path(
                    file_path_str,
                    domain.domain.as_str(),
                    domain.namespaces,
                ) && seen.insert(lang_id.clone())
                {
                    languages.push(lang_id);
                }
            }
        }

        languages.sort_by_key(|a| a.to_string());
        languages
    }
}

impl<T: EmbeddedAssets> I18nModuleDescriptor for EmbeddedI18nModule<T> {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl<T: EmbeddedAssets> I18nModule for EmbeddedI18nModule<T> {
    fn create_localizer(&self) -> Box<dyn Localizer> {
        Box::new(EmbeddedLocalizer::<T>::new(self.data))
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        self.data.supported_languages.contains(lang).then(|| {
            T::resource_plan_for_language(lang).unwrap_or_else(|| self.data.resource_plan())
        })
    }
}
