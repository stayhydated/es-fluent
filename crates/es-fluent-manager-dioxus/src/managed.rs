use crate::{
    BridgeOwner, DioxusGlobalLocalizerError, DioxusInitError, GlobalLocalizerMode,
    active_bridge_owner, global_bridge_install_lock,
};
use es_fluent::{
    FluentValue, GlobalLocalizationError, replace_custom_localizer_with_domain,
    try_set_custom_localizer_with_domain,
};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use unic_langid::LanguageIdentifier;

static ACTIVE_BRIDGE_MANAGER: OnceLock<RwLock<Option<Arc<FluentManager>>>> = OnceLock::new();

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
        install_manager_bridge(Arc::clone(&self.manager), mode)
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

pub(crate) fn install_manager_bridge(
    manager: Arc<FluentManager>,
    mode: GlobalLocalizerMode,
) -> Result<(), DioxusGlobalLocalizerError> {
    let _guard = global_bridge_install_lock();
    let requested_owner = BridgeOwner::Client(Arc::as_ptr(&manager) as usize);
    let mut owner = active_bridge_owner().write();

    match *owner {
        Some(active_owner)
            if active_owner == requested_owner && mode != GlobalLocalizerMode::ReplaceExisting =>
        {
            *active_bridge_manager().write() = Some(manager);
            return Ok(());
        },
        Some(active_owner) if mode != GlobalLocalizerMode::ReplaceExisting => {
            return Err(DioxusGlobalLocalizerError::owner_conflict(
                active_owner,
                requested_owner,
                mode,
            ));
        },
        _ => {},
    }

    let previous_manager = active_bridge_manager().read().clone();
    *active_bridge_manager().write() = Some(Arc::clone(&manager));

    if let Err(error) = install_manager_bridge_callback(mode) {
        *active_bridge_manager().write() = previous_manager;
        return Err(error.into());
    }

    *owner = Some(requested_owner);
    Ok(())
}

fn active_bridge_manager() -> &'static RwLock<Option<Arc<FluentManager>>> {
    ACTIVE_BRIDGE_MANAGER.get_or_init(|| RwLock::new(None))
}

fn install_manager_bridge_callback(
    mode: GlobalLocalizerMode,
) -> Result<(), GlobalLocalizationError> {
    let bridge =
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            let manager = active_bridge_manager().read().clone();
            manager.and_then(|manager| match domain {
                Some(domain) => manager.localize_in_domain(domain, id, args),
                None => manager.localize(id, args),
            })
        };

    match mode {
        GlobalLocalizerMode::ErrorIfAlreadySet | GlobalLocalizerMode::ReuseIfSameOwner => {
            try_set_custom_localizer_with_domain(bridge)
        },
        GlobalLocalizerMode::ReplaceExisting => {
            tracing::debug!(
                "replacing the process-global Fluent custom localizer with the Dioxus i18n manager"
            );
            replace_custom_localizer_with_domain(bridge);
            Ok(())
        },
    }
}
