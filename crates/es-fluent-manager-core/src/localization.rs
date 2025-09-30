use fluent_bundle::FluentValue;
use std::collections::HashMap;
use thiserror::Error;
use unic_langid::LanguageIdentifier;

#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("Language '{0}' is not supported by this module")]
    LanguageNotSupported(LanguageIdentifier),
    #[error("An underlying localization backend error occurred: {0}")]
    BackendError(#[from] anyhow::Error),
}

pub trait Localizer: Send + Sync {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError>;
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String>;
}

pub trait I18nModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn create_localizer(&self) -> Box<dyn Localizer>;
}

inventory::collect!(&'static dyn I18nModule);

#[derive(Default)]
pub struct FluentManager {
    localizers: Vec<Box<dyn Localizer>>,
}

impl Clone for FluentManager {
    fn clone(&self) -> Self {
        Self::new_with_discovered_modules()
    }
}

impl FluentManager {
    pub fn new_with_discovered_modules() -> Self {
        let mut manager = Self::default();
        for module in inventory::iter::<&'static dyn I18nModule>() {
            log::info!("Discovered and loading i18n module: {}", module.name());
            manager.localizers.push(module.create_localizer());
        }
        manager
    }

    pub fn select_language(&mut self, lang: &LanguageIdentifier) {
        for localizer in &self.localizers {
            if let Err(e) = localizer.select_language(lang) {
                log::warn!("Module failed to set language '{}': {}", lang, e);
            }
        }
    }

    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for localizer in &self.localizers {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }
}
