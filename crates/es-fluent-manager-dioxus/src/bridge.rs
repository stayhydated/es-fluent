use es_fluent::{
    CustomLocalizerGeneration, GlobalLocalizationError, current_custom_localizer_generation,
};
#[cfg(test)]
use es_fluent::{
    CustomLocalizerSnapshot, custom_localizer_snapshot, restore_custom_localizer_snapshot,
};
#[cfg(any(feature = "client", feature = "ssr"))]
use es_fluent::{FluentValue, try_set_custom_localizer_with_domain_and_generation};
#[cfg(feature = "client")]
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
#[cfg(any(feature = "client", feature = "ssr"))]
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DioxusGlobalLocalizerOwner {
    Client,
    Ssr,
}

#[derive(Clone, Debug)]
pub enum DioxusGlobalLocalizerError {
    OwnerConflict {
        active: DioxusGlobalLocalizerOwner,
        requested: DioxusGlobalLocalizerOwner,
    },
    Global(Arc<GlobalLocalizationError>),
}

impl DioxusGlobalLocalizerError {
    fn owner_conflict(
        active: DioxusGlobalLocalizerOwner,
        requested: DioxusGlobalLocalizerOwner,
    ) -> Self {
        Self::OwnerConflict { active, requested }
    }
}

impl std::fmt::Display for DioxusGlobalLocalizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Client,
                requested: DioxusGlobalLocalizerOwner::Client,
            } => f.write_str(
                "a different Dioxus client manager already owns the global Fluent localizer",
            ),
            Self::OwnerConflict { active, requested } => write!(
                f,
                "the Dioxus global Fluent localizer is already owned by the {active} bridge; requested the {requested} bridge",
            ),
            Self::Global(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DioxusGlobalLocalizerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OwnerConflict { .. } => None,
            Self::Global(error) => Some(error.as_ref()),
        }
    }
}

impl From<GlobalLocalizationError> for DioxusGlobalLocalizerError {
    fn from(value: GlobalLocalizationError) -> Self {
        Self::Global(Arc::new(value))
    }
}

impl std::fmt::Display for DioxusGlobalLocalizerOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client => f.write_str("client"),
            Self::Ssr => f.write_str("SSR"),
        }
    }
}

#[derive(Clone)]
enum InstalledBridge {
    #[cfg(feature = "client")]
    Client {
        manager: Arc<FluentManager>,
        generation: CustomLocalizerGeneration,
    },
    #[cfg(feature = "ssr")]
    Ssr {
        generation: CustomLocalizerGeneration,
    },
}

impl InstalledBridge {
    fn owner(&self) -> DioxusGlobalLocalizerOwner {
        match self {
            #[cfg(feature = "client")]
            Self::Client { .. } => DioxusGlobalLocalizerOwner::Client,
            #[cfg(feature = "ssr")]
            Self::Ssr { .. } => DioxusGlobalLocalizerOwner::Ssr,
        }
    }

    fn generation(&self) -> CustomLocalizerGeneration {
        match self {
            #[cfg(feature = "client")]
            Self::Client { generation, .. } => *generation,
            #[cfg(feature = "ssr")]
            Self::Ssr { generation } => *generation,
        }
    }

    fn is_current_custom_localizer(&self) -> bool {
        current_custom_localizer_generation() == Some(self.generation())
    }

    #[cfg(feature = "client")]
    fn is_same_client_manager(&self, manager: &Arc<FluentManager>) -> bool {
        matches!(self, Self::Client { manager: current, .. } if Arc::ptr_eq(current, manager))
    }
}

static GLOBAL_BRIDGE_INSTALL_LOCK: Mutex<()> = Mutex::new(());
static INSTALLED_BRIDGE: OnceLock<RwLock<Option<InstalledBridge>>> = OnceLock::new();
#[cfg(test)]
static TEST_INITIAL_CUSTOM_LOCALIZER: OnceLock<CustomLocalizerSnapshot> = OnceLock::new();

pub(crate) fn global_bridge_install_lock() -> MutexGuard<'static, ()> {
    GLOBAL_BRIDGE_INSTALL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn installed_bridge() -> &'static RwLock<Option<InstalledBridge>> {
    INSTALLED_BRIDGE.get_or_init(|| RwLock::new(None))
}

fn discard_stale_bridge(bridge: &mut Option<InstalledBridge>) {
    if bridge
        .as_ref()
        .is_some_and(|bridge| !bridge.is_current_custom_localizer())
    {
        *bridge = None;
    }
}

#[cfg(test)]
pub(crate) fn reset_global_bridge_for_tests() {
    let _guard = global_bridge_install_lock();
    let initial = TEST_INITIAL_CUSTOM_LOCALIZER
        .get_or_init(custom_localizer_snapshot)
        .clone();
    restore_custom_localizer_snapshot(initial);
    *installed_bridge().write() = None;
}

#[cfg(feature = "client")]
pub(crate) fn install_client_bridge(
    manager: Arc<FluentManager>,
) -> Result<(), DioxusGlobalLocalizerError> {
    let _guard = global_bridge_install_lock();
    let mut bridge = installed_bridge().write();
    install_client_bridge_locked(&mut bridge, manager).map(|_| ())
}

#[cfg(feature = "client")]
fn install_client_bridge_locked(
    bridge: &mut Option<InstalledBridge>,
    manager: Arc<FluentManager>,
) -> Result<CustomLocalizerGeneration, DioxusGlobalLocalizerError> {
    discard_stale_bridge(bridge);

    match bridge.as_ref() {
        Some(active) if active.is_same_client_manager(&manager) => return Ok(active.generation()),
        Some(active) => {
            return Err(DioxusGlobalLocalizerError::owner_conflict(
                active.owner(),
                DioxusGlobalLocalizerOwner::Client,
            ));
        },
        None => {},
    }

    let callback_manager = Arc::clone(&manager);
    let generation = try_set_custom_localizer_with_domain_and_generation(
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            localize_with_manager(&callback_manager, domain, id, args)
        },
    )?;

    *bridge = Some(InstalledBridge::Client {
        manager,
        generation,
    });
    Ok(generation)
}

#[cfg(feature = "ssr")]
pub(crate) fn install_ssr_bridge<F>(callback: F) -> Result<(), DioxusGlobalLocalizerError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    let _guard = global_bridge_install_lock();
    let mut bridge = installed_bridge().write();
    install_ssr_bridge_locked(&mut bridge, callback).map(|_| ())
}

#[cfg(feature = "ssr")]
fn install_ssr_bridge_locked<F>(
    bridge: &mut Option<InstalledBridge>,
    callback: F,
) -> Result<CustomLocalizerGeneration, DioxusGlobalLocalizerError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    discard_stale_bridge(bridge);

    match bridge.as_ref() {
        Some(active) if active.owner() == DioxusGlobalLocalizerOwner::Ssr => {
            return Ok(active.generation());
        },
        Some(active) => {
            return Err(DioxusGlobalLocalizerError::owner_conflict(
                active.owner(),
                DioxusGlobalLocalizerOwner::Ssr,
            ));
        },
        None => {},
    }

    let generation = try_set_custom_localizer_with_domain_and_generation(callback)?;
    *bridge = Some(InstalledBridge::Ssr { generation });
    Ok(generation)
}

#[cfg(feature = "client")]
fn localize_with_manager<'a>(
    manager: &FluentManager,
    domain: Option<&str>,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> Option<String> {
    let message = match domain {
        Some(domain) => manager.localize_in_domain(domain, id, args),
        None => manager.localize(id, args),
    };

    match message {
        Some(message) => Some(message),
        None => {
            match domain {
                Some(domain) => tracing::warn!(domain, message_id = id, "missing Fluent message"),
                None => tracing::warn!(message_id = id, "missing Fluent message"),
            }
            Some(id.to_string())
        },
    }
}
