//! This module provides the core types for managing translations.

use es_fluent_derive_core::EsFluentError;
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

pub type LocalizationError = EsFluentError;

pub trait Localizer: Send + Sync {
    /// Selects a language for the localizer.
    fn select_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> es_fluent_derive_core::EsFluentResult<()>;
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
    localizers: Vec<(&'static str, Box<dyn Localizer>)>,
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
            manager
                .localizers
                .push((module.name(), module.create_localizer()));
        }
        manager
    }

    /// Selects a language for all localizers.
    pub fn select_language(&self, lang: &LanguageIdentifier) {
        let mut any_selected = false;

        for (name, localizer) in &self.localizers {
            match localizer.select_language(lang) {
                Ok(()) => {
                    any_selected = true;
                },
                Err(e) => {
                    tracing::debug!("Module '{}' failed to set language '{}': {}", name, lang, e);
                },
            }
        }

        if !any_selected {
            tracing::warn!("No i18n modules support language '{}'", lang);
        }
    }

    /// Localizes a message by its ID.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (_, localizer) in &self.localizers {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use unic_langid::langid;

    static SELECT_OK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SELECT_ERR_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct ModuleOk;
    struct ModuleErr;

    struct LocalizerOk;
    struct LocalizerErr;

    impl Localizer for LocalizerOk {
        fn select_language(&self, _lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            SELECT_OK_CALLS.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            match id {
                "from-ok" => Some("ok-value".to_string()),
                _ => None,
            }
        }
    }

    impl Localizer for LocalizerErr {
        fn select_language(&self, lang: &LanguageIdentifier) -> Result<(), LocalizationError> {
            SELECT_ERR_CALLS.fetch_add(1, Ordering::Relaxed);
            Err(LocalizationError::LanguageNotSupported(lang.clone()))
        }

        fn localize<'a>(
            &self,
            id: &str,
            _args: Option<&HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String> {
            if id == "from-err" {
                Some("err-value".to_string())
            } else {
                None
            }
        }
    }

    impl I18nModule for ModuleOk {
        fn name(&self) -> &'static str {
            "module-ok"
        }

        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(LocalizerOk)
        }
    }

    impl I18nModule for ModuleErr {
        fn name(&self) -> &'static str {
            "module-err"
        }

        fn create_localizer(&self) -> Box<dyn Localizer> {
            Box::new(LocalizerErr)
        }
    }

    static MODULE_OK: ModuleOk = ModuleOk;
    static MODULE_ERR: ModuleErr = ModuleErr;

    inventory::submit! {
        &MODULE_OK as &dyn I18nModule
    }

    inventory::submit! {
        &MODULE_ERR as &dyn I18nModule
    }

    #[test]
    fn manager_select_language_calls_all_localizers() {
        SELECT_OK_CALLS.store(0, Ordering::Relaxed);
        SELECT_ERR_CALLS.store(0, Ordering::Relaxed);

        let manager = FluentManager::new_with_discovered_modules();
        manager.select_language(&langid!("en-US"));

        assert!(SELECT_OK_CALLS.load(Ordering::Relaxed) >= 1);
        assert!(SELECT_ERR_CALLS.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn manager_localize_returns_first_matching_message() {
        let manager = FluentManager::new_with_discovered_modules();
        assert_eq!(
            manager.localize("from-ok", None),
            Some("ok-value".to_string())
        );
        assert_eq!(
            manager.localize("from-err", None),
            Some("err-value".to_string())
        );
        assert_eq!(manager.localize("missing", None), None);
    }

    #[test]
    fn manager_clone_rebuilds_discovered_modules() {
        let manager = FluentManager::new_with_discovered_modules();
        let clone = manager.clone();
        assert_eq!(
            clone.localize("from-ok", None),
            Some("ok-value".to_string())
        );
    }

    #[test]
    fn manager_select_language_with_only_failing_localizers_covers_warn_path() {
        SELECT_ERR_CALLS.store(0, Ordering::Relaxed);

        let manager = FluentManager {
            localizers: vec![("module-err", Box::new(LocalizerErr))],
        };
        manager.select_language(&langid!("en-US"));

        assert_eq!(SELECT_ERR_CALLS.load(Ordering::Relaxed), 1);
    }
}
