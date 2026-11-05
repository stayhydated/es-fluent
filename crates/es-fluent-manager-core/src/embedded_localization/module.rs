use crate::{
    asset_localization::{I18nModuleDescriptor, LocaleLoadReport, ModuleData, ModuleResourceSpec},
    localization::{
        I18nModule, LocaleResources, LocalizationError, LocalizationErrorResult, Localizer,
        ModuleLocalizer, log_unready_locale,
    },
};
use unic_langid::LanguageIdentifier;

use super::EmbeddedLanguage;

/// A module whose locale resources are compiled into the binary.
pub struct EmbeddedI18nModule {
    data: &'static ModuleData,
    languages: &'static [EmbeddedLanguage],
}

impl EmbeddedI18nModule {
    /// Creates a module from its metadata and embedded locales.
    pub const fn new(data: &'static ModuleData, languages: &'static [EmbeddedLanguage]) -> Self {
        Self { data, languages }
    }

    fn language(&self, lang: &LanguageIdentifier) -> Option<&'static EmbeddedLanguage> {
        self.languages
            .iter()
            .find(|language| &language.language == lang)
    }

    pub(super) fn load_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> LocalizationErrorResult<LocaleResources> {
        let Some(language) = self.language(lang) else {
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        };

        let plan = language.resource_plan();
        let mut report = LocaleLoadReport::from_plan(&plan);
        let mut resources = Vec::with_capacity(plan.len());
        for (spec, embedded) in plan.into_iter().zip(language.resources) {
            match embedded.resource() {
                Ok(resource) => {
                    report.mark_loaded(spec.key.clone());
                    resources.push((spec.key, resource));
                },
                Err(error) => {
                    tracing::debug!("{}", error);
                    report.record_error(error);
                },
            }
        }

        if !report.is_ready() {
            log_unready_locale(self.data, lang, &report);
            return Err(LocalizationError::LanguageNotSupported(lang.clone()));
        }
        Ok(resources)
    }
}

impl I18nModuleDescriptor for EmbeddedI18nModule {
    fn data(&self) -> &'static ModuleData {
        self.data
    }
}

impl I18nModule for EmbeddedI18nModule {
    fn create_localizer(
        &self,
        lang: &LanguageIdentifier,
    ) -> LocalizationErrorResult<Box<dyn Localizer>> {
        let localizer =
            ModuleLocalizer::new(self.data, lang, |candidate| self.load_language(candidate))?;
        Ok(Box::new(localizer))
    }

    fn resource_plan_for_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> Option<Vec<ModuleResourceSpec>> {
        if !self.data.supported_languages.contains(lang) {
            return None;
        }
        self.language(lang).map(EmbeddedLanguage::resource_plan)
    }
}
