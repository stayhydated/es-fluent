//! This module provides the core types for managing translations.

use fluent_bundle::FluentValue;
use std::collections::HashMap;
use thiserror::Error;
use unic_langid::LanguageIdentifier;

/// An error that can occur when localizing a message.
#[derive(Debug, Error)]
pub enum LocalizationError {
    /// An error that occurs when a language is not supported.
    #[error("Language '{0}' is not supported by this module")]
    LanguageNotSupported(LanguageIdentifier),
    /// An error that occurs in the localization backend.
    #[error("An underlying localization backend error occurred: {0}")]
    BackendError(#[from] anyhow::Error),
}

pub trait Localizer: Send + Sync {
    /// Selects a language for the localizer.
    fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError>;
    /// Localizes a message by its ID.
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String>;
}

pub trait I18nModule: Send + Sync {
    /// Returns the name of the module.
    fn name(&self) -> &'static str;
    /// Creates a localizer for the module.
    fn create_localizer(&self) -> Box<dyn Localizer>;
}

inventory::collect!(&'static dyn I18nModule);

/// A manager for Fluent translations.
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
    /// Creates a new `FluentManager` with discovered i18n modules.
    pub fn new_with_discovered_modules() -> Self {
        let mut manager = Self::default();
        for module in inventory::iter::<&'static dyn I18nModule>() {
            tracing::info!("Discovered and loading i18n module: {}", module.name());
            manager.localizers.push(module.create_localizer());
        }
        manager
    }

    /// Selects a language for all localizers.
    pub fn select_language(&self, lang: &LanguageIdentifier) {
        for localizer in &self.localizers {
            if let Err(e) = localizer.select_language(lang) {
                tracing::warn!("Module failed to set language '{}': {}", lang, e);
            }
        }
    }

    /// Localizes a message by its ID.
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
