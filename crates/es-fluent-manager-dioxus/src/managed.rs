use crate::DioxusInitError;
#[cfg(feature = "client")]
use crate::{DioxusGlobalLocalizerError, bridge::install_client_bridge};
use es_fluent::{FluentValue, GlobalLocalizationError};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

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
            requested_language: Arc::new(RwLock::new(lang)),
        })
    }

    pub(crate) fn manager(&self) -> &Arc<FluentManager> {
        &self.manager
    }

    pub fn requested_language(&self) -> LanguageIdentifier {
        self.requested_language.read().clone()
    }

    pub fn select_language<L: Into<LanguageIdentifier>>(&self, lang: L) {
        self.try_select_language(lang)
            .unwrap_or_else(|error| panic!("failed to select Dioxus i18n language: {error}"));
    }

    pub fn try_select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        let lang = lang.into();
        self.manager
            .select_language(&lang)
            .map_err(GlobalLocalizationError::from)?;
        *self.requested_language.write() = lang;
        Ok(())
    }

    pub fn select_language_strict<L: Into<LanguageIdentifier>>(&self, lang: L) {
        self.try_select_language_strict(lang)
            .unwrap_or_else(|error| {
                panic!("failed to strictly select Dioxus i18n language: {error}")
            });
    }

    pub fn try_select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), GlobalLocalizationError> {
        let lang = lang.into();
        self.manager
            .select_language_strict(&lang)
            .map_err(GlobalLocalizationError::from)?;
        *self.requested_language.write() = lang;
        Ok(())
    }

    #[cfg(feature = "client")]
    pub fn install_client_process_global_bridge(&self) -> Result<(), DioxusGlobalLocalizerError> {
        install_client_bridge(Arc::clone(&self.manager))
    }

    pub fn try_localize<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize(id.as_ref(), args)
    }

    pub fn localize<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let id = id.as_ref();
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
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        let domain = domain.as_ref();
        let id = id.as_ref();
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
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager
            .localize_in_domain(domain.as_ref(), id.as_ref(), args)
    }
}
