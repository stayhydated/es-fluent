use crate::DioxusInitError;
use es_fluent::{FluentLocalizer, FluentLocalizerExt, FluentMessage, FluentValue};
use es_fluent_manager_core::{
    DiscoveredRuntimeI18nModules, FluentManager, LanguageSelectionPolicy, LocalizationError,
};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

/// Dioxus-owned localization manager.
///
/// Equality is identity equality: two values compare equal only when they share
/// the same internal manager, requested-language state, and selection lock, not
/// when they merely contain equivalent modules or requested languages.
#[derive(Clone)]
pub struct ManagedI18n {
    manager: Arc<FluentManager>,
    requested_language: Arc<RwLock<LanguageIdentifier>>,
    selection_lock: Arc<Mutex<()>>,
}

impl PartialEq for ManagedI18n {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.manager, &other.manager)
            && Arc::ptr_eq(&self.requested_language, &other.requested_language)
            && Arc::ptr_eq(&self.selection_lock, &other.selection_lock)
    }
}

impl Eq for ManagedI18n {}

impl ManagedI18n {
    pub fn new_with_discovered_modules<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        Self::new_with_discovered_modules_with_policy(lang, LanguageSelectionPolicy::BestEffort)
    }

    pub fn new_with_discovered_modules_strict<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        Self::new_with_discovered_modules_with_policy(lang, LanguageSelectionPolicy::Strict)
    }

    pub fn new_with_discovered_modules_with_policy<L: Into<LanguageIdentifier>>(
        lang: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(DioxusInitError::module_discovery)?;
        Self::from_manager_with_policy(manager, lang, selection_policy)
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn new_with_cached_modules_strict<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        Self::new_with_cached_modules_with_policy(modules, lang, LanguageSelectionPolicy::Strict)
    }

    #[cfg(test)]
    pub(crate) fn new_with_cached_modules<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        Self::new_with_cached_modules_with_policy(
            modules,
            lang,
            LanguageSelectionPolicy::BestEffort,
        )
    }

    pub(crate) fn new_with_cached_modules_with_policy<L: Into<LanguageIdentifier>>(
        modules: &DiscoveredRuntimeI18nModules,
        lang: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusInitError> {
        let lang = lang.into();
        let manager = FluentManager::from_discovered_modules(modules);
        Self::from_manager_with_policy(manager, lang, selection_policy)
    }

    fn from_manager_with_policy(
        manager: FluentManager,
        lang: LanguageIdentifier,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<Self, DioxusInitError> {
        match selection_policy {
            LanguageSelectionPolicy::BestEffort => manager.select_language(&lang),
            LanguageSelectionPolicy::Strict => manager.select_language_strict(&lang),
        }
        .map_err(DioxusInitError::language_selection)?;

        Ok(Self {
            manager: Arc::new(manager),
            requested_language: Arc::new(RwLock::new(lang)),
            selection_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        let _selection = self.selection_lock.lock();
        self.requested_language.read().clone()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::BestEffort)
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::Strict)
    }

    fn select_language_with_policy<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
        selection_policy: LanguageSelectionPolicy,
    ) -> Result<(), LocalizationError> {
        let lang = lang.into();
        let _selection = self.selection_lock.lock();
        match selection_policy {
            LanguageSelectionPolicy::BestEffort => self.manager.select_language(&lang),
            LanguageSelectionPolicy::Strict => self.manager.select_language_strict(&lang),
        }?;
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
        let _selection = self.selection_lock.lock();
        self.manager.localize(id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        let _selection = self.selection_lock.lock();
        self.manager.localize_in_domain(domain, id, args)
    }
}
