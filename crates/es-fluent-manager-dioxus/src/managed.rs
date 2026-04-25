use crate::DioxusInitError;
use es_fluent::{
    FluentValue, GlobalLocalizationError, replace_custom_localizer_with_domain,
    try_set_custom_localizer_with_domain,
};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use unic_langid::LanguageIdentifier;

static MANAGER_BRIDGE_INSTALLED: AtomicBool = AtomicBool::new(false);
static MANAGER_BRIDGE_INSTALL_LOCK: Mutex<()> = Mutex::new(());
static ACTIVE_BRIDGE_MANAGER: OnceLock<RwLock<Option<Arc<FluentManager>>>> = OnceLock::new();

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
        match self.manager.localize(id, args) {
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
        match self.manager.localize_in_domain(domain, id, args) {
            Some(value) => value,
            None => {
                tracing::warn!(domain, message_id = id, "missing Fluent message");
                id.to_string()
            },
        }
    }
}

pub(crate) fn install_manager_bridge(
    manager: Arc<FluentManager>,
    mode: GlobalLocalizerMode,
) -> Result<(), GlobalLocalizationError> {
    install_manager_bridge_once(mode)?;
    *active_bridge_manager().write() = Some(manager);
    Ok(())
}

fn active_bridge_manager() -> &'static RwLock<Option<Arc<FluentManager>>> {
    ACTIVE_BRIDGE_MANAGER.get_or_init(|| RwLock::new(None))
}

fn install_manager_bridge_once(mode: GlobalLocalizerMode) -> Result<(), GlobalLocalizationError> {
    if MANAGER_BRIDGE_INSTALLED.load(Ordering::Acquire) {
        return Ok(());
    }

    let _guard = MANAGER_BRIDGE_INSTALL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if MANAGER_BRIDGE_INSTALLED.load(Ordering::Acquire) {
        return Ok(());
    }

    install_manager_bridge_callback(mode)?;
    MANAGER_BRIDGE_INSTALLED.store(true, Ordering::Release);
    Ok(())
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
        GlobalLocalizerMode::ErrorIfAlreadySet => try_set_custom_localizer_with_domain(bridge),
        GlobalLocalizerMode::ReplaceExisting => {
            tracing::debug!(
                "replacing the process-global Fluent custom localizer with the Dioxus i18n manager"
            );
            replace_custom_localizer_with_domain(bridge);
            Ok(())
        },
    }
}
