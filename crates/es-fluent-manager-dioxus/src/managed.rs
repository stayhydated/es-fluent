use crate::{
    DioxusGlobalLocalizerError, DioxusInitError, GlobalLocalizerMode, bridge::install_client_bridge,
};
use es_fluent::{FluentValue, GlobalLocalizationError};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[derive(Clone)]
pub struct ManagedI18n {
    manager: Arc<FluentManager>,
    active_language: Arc<RwLock<LanguageIdentifier>>,
}

impl ManagedI18n {
    pub fn new_with_discovered_modules<L: Into<LanguageIdentifier>>(lang: L) -> Self {
        Self::try_new_with_discovered_modules(lang)
            .unwrap_or_else(|error| panic!("failed to initialize Dioxus i18n manager: {error}"))
    }

    pub fn try_new_with_discovered_modules<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, DioxusInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(DioxusInitError::module_discovery)?;
        manager
            .select_language(&lang)
            .map_err(GlobalLocalizationError::from)
            .map_err(DioxusInitError::language_selection)?;

        Ok(Self {
            manager: Arc::new(manager),
            active_language: Arc::new(RwLock::new(lang)),
        })
    }

    /// Returns the underlying manager as an escape hatch for integrations.
    ///
    /// Do not use this to switch languages in Dioxus UI code. `FluentManager`
    /// has interior mutable language state, so calling selection methods on the
    /// returned manager bypasses `ManagedI18n::active_language()` and any
    /// Dioxus signal held by `DioxusI18n`. Use `select_language(...)` or
    /// `select_language_strict(...)` when the tracked language should remain
    /// synchronized.
    pub fn manager(&self) -> Arc<FluentManager> {
        Arc::clone(&self.manager)
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.active_language()
    }

    pub fn active_language(&self) -> LanguageIdentifier {
        self.active_language.read().clone()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        let lang = lang.into();
        self.manager
            .select_language(&lang)
            .map_err(GlobalLocalizationError::from)?;
        *self.active_language.write() = lang;
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        let lang = lang.into();
        self.manager
            .select_language_strict(&lang)
            .map_err(GlobalLocalizationError::from)?;
        *self.active_language.write() = lang;
        Ok(())
    }

    pub fn install_global_localizer(
        &self,
        mode: GlobalLocalizerMode,
    ) -> Result<(), DioxusGlobalLocalizerError> {
        install_client_bridge(Arc::clone(&self.manager), mode)
    }

    pub fn try_localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize(id, args)
    }

    pub fn localize<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> String {
        match self.try_localize(id, args) {
            Some(value) => value,
            None => {
                tracing::warn!(message_id = id, "missing Fluent message");
                id.to_string()
            },
        }
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        match self.try_localize_in_domain(domain, id, args) {
            Some(value) => value,
            None => {
                tracing::warn!(domain, message_id = id, "missing Fluent message");
                id.to_string()
            },
        }
    }

    pub fn try_localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize_in_domain(domain, id, args)
    }
}
