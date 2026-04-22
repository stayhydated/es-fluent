use crate::DioxusInitError;
use es_fluent::{
    FluentValue, GlobalLocalizationError, replace_custom_localizer_with_domain,
    try_set_custom_localizer_with_domain,
};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GlobalLocalizerMode {
    #[default]
    ErrorIfAlreadySet,
    ReplaceExisting,
}

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
            .map_err(DioxusInitError::ModuleDiscovery)?;
        manager
            .select_language(&lang)
            .map_err(GlobalLocalizationError::from)
            .map_err(DioxusInitError::LanguageSelection)?;

        Ok(Self {
            manager: Arc::new(manager),
            active_language: Arc::new(RwLock::new(lang)),
        })
    }

    pub fn manager(&self) -> Arc<FluentManager> {
        Arc::clone(&self.manager)
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

    pub fn install_global_localizer(
        &self,
        mode: GlobalLocalizerMode,
    ) -> Result<(), GlobalLocalizationError> {
        install_manager_bridge(Arc::clone(&self.manager), mode)
    }

    pub fn localize<'a>(&self, id: &str, args: Option<&HashMap<&str, FluentValue<'a>>>) -> String {
        self.manager
            .localize(id, args)
            .unwrap_or_else(|| id.to_string())
    }

    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        self.manager
            .localize_in_domain(domain, id, args)
            .unwrap_or_else(|| id.to_string())
    }
}

pub(crate) fn install_manager_bridge(
    manager: Arc<FluentManager>,
    mode: GlobalLocalizerMode,
) -> Result<(), GlobalLocalizationError> {
    let bridge = move |domain: Option<&str>,
                       id: &str,
                       args: Option<&HashMap<&str, FluentValue<'_>>>| match domain
    {
        Some(domain) => manager.localize_in_domain(domain, id, args),
        None => manager.localize(id, args),
    };

    match mode {
        GlobalLocalizerMode::ErrorIfAlreadySet => try_set_custom_localizer_with_domain(bridge),
        GlobalLocalizerMode::ReplaceExisting => {
            replace_custom_localizer_with_domain(bridge);
            Ok(())
        },
    }
}
