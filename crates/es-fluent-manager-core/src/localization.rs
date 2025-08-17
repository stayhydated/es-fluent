// In: crates/es-fluent-manager-core/src/localization.rs

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

/// The core trait for any localization backend.
pub trait Localizer: Send + Sync {
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError>;
    fn localize<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>;
}

/// A trait for a discoverable module that can create a `Localizer`.
pub trait I18nModule: Send + Sync {
    fn name(&self) -> &'static str;
    fn create_localizer(&self) -> Box<dyn Localizer>;
}

inventory::collect!(&'static dyn I18nModule);

/// The core struct for managing localization.
/// This struct holds the localizers and provides methods for language selection and localization.
#[derive(Default)]
pub struct FluentManager {
    localizers: Vec<Box<dyn Localizer>>,
}

impl Clone for FluentManager {
    fn clone(&self) -> Self {
        // This is a simplified clone implementation.
        // In a real implementation, we might need to recreate the localizers.
        // For now, we'll create a new manager with the same modules.
        Self::new_with_discovered_modules()
    }
}

impl FluentManager {
    /// Creates a new `FluentManager` and automatically discovers registered i18n modules.
    pub fn new_with_discovered_modules() -> Self {
        let mut manager = Self::default();
        for module in inventory::iter::<&'static dyn I18nModule>() {
            log::info!("Discovered and loading i18n module: {}", module.name());
            manager.localizers.push(module.create_localizer());
        }
        manager
    }

    /// Selects the language for all registered localizers.
    /// Errors from individual localizers are logged as warnings.
    pub fn select_language(&mut self, lang: &LanguageIdentifier) {
        for localizer in &self.localizers {
            if let Err(e) = localizer.select_language(lang) {
                log::warn!("Module failed to set language '{}': {}", lang, e);
            }
        }
    }

    /// Attempts to localize a message ID with optional arguments.
    /// Returns the first successful localization from any localizer, or `None` if not found.
    pub fn localize<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String> {
        for localizer in &self.localizers {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }
}