// In: crates/es-fluent/src/localization.rs

use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, OnceLock};
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

#[derive(Default)]
pub struct LocalizationContext {
    localizers: Vec<Box<dyn Localizer>>,
}

impl LocalizationContext {
    pub fn new_with_discovered_modules() -> Self {
        let mut context = Self::default();
        for module in inventory::iter::<&'static dyn I18nModule>() {
            log::info!("Discovered and loading i18n module: {}", module.name());
            context.localizers.push(module.create_localizer());
        }
        context
    }

    pub fn select_language(&mut self, lang: &LanguageIdentifier) {
        for localizer in &self.localizers {
            if let Err(e) = localizer.select_language(lang) {
                log::warn!("Module failed to set language '{}': {}", lang, e);
            }
        }
    }

    fn get_translation<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String> {
        for localizer in &self.localizers {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }
}

// --- Global Thread-Safe Implementation ---

static CONTEXT: OnceLock<Arc<RwLock<LocalizationContext>>> = OnceLock::new();

/// Sets the global localization context.
/// Should be called once at application startup.
pub fn set_context(context: LocalizationContext) {
    CONTEXT.set(Arc::new(RwLock::new(context)))
        .map_err(|_| "Localization context already set")
        .expect("Failed to set localization context");
}

/// Provides safe, scoped access to the localization context for mutation.
pub fn with_context<F, R>(f: F) -> R
where
    F: FnOnce(&mut LocalizationContext) -> R
{
    let context_arc = CONTEXT.get()
        .expect("Localization context not set. Call es_fluent::localization::set_context() at startup.");

    let mut context = context_arc.write()
        .expect("Failed to acquire write lock on localization context");

    f(&mut *context)
}

/// The global localization function used by generated code.
pub fn localize<'a>(id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> String {
    let context_arc = CONTEXT.get()
        .expect("Localization context not set. Call es_fluent::localization::set_context() at startup.");

    let context = context_arc.read()
        .expect("Failed to acquire read lock on localization context");

    context.get_translation(id, args)
        .unwrap_or_else(|| {
            log::warn!("Translation for '{}' not found or context not set.", id);
            id.to_string()
        })
}
