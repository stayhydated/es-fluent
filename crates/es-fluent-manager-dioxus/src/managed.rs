use crate::DioxusInitError;
use es_fluent::__private::FluentLocalizerExt;
use es_fluent::{FluentLocalizer, FluentMessage, FluentValue};
use es_fluent_manager_core::{DiscoveredRuntimeI18nModules, FluentManager, LocalizationError};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// Dioxus-owned localization manager.
///
/// Equality is identity equality: two values compare equal only when they share
/// the same internal manager and requested-language state, not when they merely
/// contain equivalent modules or requested languages.
#[derive(Clone)]
pub struct ManagedI18n {
    manager: Arc<FluentManager>,
    requested_language: Arc<RwLock<LanguageIdentifier>>,
}

impl PartialEq for ManagedI18n {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.manager, &other.manager)
            && Arc::ptr_eq(&self.requested_language, &other.requested_language)
    }
}

impl Eq for ManagedI18n {}

impl ManagedI18n {
    pub fn new_with_discovered_modules<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(DioxusInitError::module_discovery)?;
        Self::from_manager(manager, lang)
    }

    pub fn new_with_cached_modules<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        let lang = lang.into();
        let manager = FluentManager::from_discovered_modules(modules);
        Self::from_manager(manager, lang)
    }

    fn from_manager(
        manager: FluentManager,
        lang: LanguageIdentifier,
    ) -> Result<Self, DioxusInitError> {
        manager
            .select_language(&lang)
            .map_err(DioxusInitError::language_selection)?;

        Ok(Self {
            manager: Arc::new(manager),
            requested_language: Arc::new(RwLock::new(lang)),
        })
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.requested_language.read().clone()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        let lang = lang.into();
        self.manager.select_language(&lang)?;
        *self.requested_language.write() = lang;
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        let lang = lang.into();
        self.manager.select_language_strict(&lang)?;
        *self.requested_language.write() = lang;
        Ok(())
    }

    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message(self, message)
    }
}

impl FluentLocalizer for ManagedI18n {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize(id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize_in_domain(domain, id, args)
    }
}
