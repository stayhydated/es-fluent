use i18n_embed::unic_langid::LanguageIdentifier;
use std::sync::{Arc, RwLock};
use std::sync::LazyLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum I18nManagerError {
    #[error("No I18n modules found in inventory")]
    NoModules,
    #[error("Module {module_name} failed: {source}")]
    ModuleError {
        module_name: String,
        #[source]
        source: anyhow::Error,
    },
    #[error("Multiple module errors: {errors}")]
    MultipleErrors { errors: String },
}

pub trait I18nModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn init(&self, requested_languages: &[LanguageIdentifier]) -> Result<(), I18nManagerError>;
    fn change_locale(&self, language: &str) -> Result<(), I18nManagerError>;
}

inventory::collect!(&'static dyn I18nModule);

pub struct I18nManager {
    current_language: Arc<RwLock<Option<String>>>,
    initialized: Arc<RwLock<bool>>,
}

impl I18nManager {
    pub fn new() -> Self {
        Self {
            current_language: Arc::new(RwLock::new(None)),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    pub fn init_all(&self, requested_languages: &[LanguageIdentifier]) -> Result<(), I18nManagerError> {
        let mut initialized = self.initialized.write().unwrap();
        if *initialized {
            return Ok(());
        }

        let modules: Vec<_> = inventory::iter::<&'static dyn I18nModule>().collect();
        if modules.is_empty() {
            return Err(I18nManagerError::NoModules);
        }

        for module in modules {
            module.init(requested_languages)?;
            log::info!("Initialized I18n module: {}", module.name());
        }

        if let Some(lang) = requested_languages.first() {
            *self.current_language.write().unwrap() = Some(lang.to_string());
        }

        *initialized = true;
        Ok(())
    }

    pub fn change_locale_all(&self, language: &str) -> Result<(), I18nManagerError> {
        let modules: Vec<_> = inventory::iter::<&'static dyn I18nModule>().collect();
        if modules.is_empty() {
            return Err(I18nManagerError::NoModules);
        }

        let mut errors = Vec::new();

        for module in modules {
            if let Err(e) = module.change_locale(language) {
                errors.push(format!("Module {}: {}", module.name(), e));
            }
        }

        if errors.is_empty() {
            *self.current_language.write().unwrap() = Some(language.to_string());
            Ok(())
        } else {
            Err(I18nManagerError::MultipleErrors {
                errors: errors.join(", "),
            })
        }
    }

    pub fn current_language(&self) -> Option<String> {
        self.current_language.read().unwrap().clone()
    }
}

pub static I18N_MANAGER: LazyLock<I18nManager> = LazyLock::new(I18nManager::new);
