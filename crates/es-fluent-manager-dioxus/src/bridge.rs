use es_fluent::GlobalLocalizationError;
#[cfg(any(feature = "client", feature = "ssr"))]
use es_fluent::try_set_custom_localizer_lookup_with_domain_and_generation;
#[cfg(any(feature = "client", feature = "ssr"))]
use es_fluent::{CustomLocalizerGeneration, current_custom_localizer_generation};
#[cfg(feature = "client")]
use es_fluent::{CustomLocalizerLookup, FluentValue};
#[cfg(all(test, any(feature = "client", feature = "ssr")))]
use es_fluent::{
    CustomLocalizerSnapshot, custom_localizer_snapshot, restore_custom_localizer_snapshot,
};
#[cfg(feature = "client")]
use es_fluent_manager_core::FluentManager;
#[cfg(any(feature = "client", feature = "ssr"))]
use parking_lot::RwLock;
#[cfg(feature = "client")]
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(any(feature = "client", feature = "ssr"))]
use std::sync::{Mutex, MutexGuard, OnceLock};

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
    ExternalReplacement {
        owner: DioxusGlobalLocalizerOwner,
    },
    Global(Arc<GlobalLocalizationError>),
}

impl DioxusGlobalLocalizerError {
    #[cfg(any(feature = "client", feature = "ssr"))]
    fn owner_conflict(
        active: DioxusGlobalLocalizerOwner,
        requested: DioxusGlobalLocalizerOwner,
    ) -> Self {
        Self::OwnerConflict { active, requested }
    }

    #[cfg(any(feature = "client", feature = "ssr"))]
    fn external_replacement(owner: DioxusGlobalLocalizerOwner) -> Self {
        Self::ExternalReplacement { owner }
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
            Self::ExternalReplacement { owner } => write!(
                f,
                "the Dioxus {owner} bridge no longer owns the global Fluent localizer",
            ),
            Self::Global(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DioxusGlobalLocalizerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OwnerConflict { .. } => None,
            Self::ExternalReplacement { .. } => None,
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
#[cfg(any(feature = "client", feature = "ssr"))]
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

#[cfg(any(feature = "client", feature = "ssr"))]
impl InstalledBridge {
    fn owner(&self) -> DioxusGlobalLocalizerOwner {
        match self {
            #[cfg(feature = "client")]
            Self::Client { .. } => DioxusGlobalLocalizerOwner::Client,
            #[cfg(feature = "ssr")]
            Self::Ssr { .. } => DioxusGlobalLocalizerOwner::Ssr,
            #[cfg(not(any(feature = "client", feature = "ssr")))]
            _ => unreachable!("no Dioxus bridge owners exist when client and ssr are disabled"),
        }
    }

    fn generation(&self) -> CustomLocalizerGeneration {
        match self {
            #[cfg(feature = "client")]
            Self::Client { generation, .. } => *generation,
            #[cfg(feature = "ssr")]
            Self::Ssr { generation } => *generation,
            #[cfg(not(any(feature = "client", feature = "ssr")))]
            _ => unreachable!("no Dioxus bridge owners exist when client and ssr are disabled"),
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

#[cfg(any(feature = "client", feature = "ssr"))]
static GLOBAL_BRIDGE_INSTALL_LOCK: Mutex<()> = Mutex::new(());
#[cfg(any(feature = "client", feature = "ssr"))]
static INSTALLED_BRIDGE: OnceLock<RwLock<Option<InstalledBridge>>> = OnceLock::new();
#[cfg(all(test, any(feature = "client", feature = "ssr")))]
static TEST_INITIAL_CUSTOM_LOCALIZER: OnceLock<CustomLocalizerSnapshot> = OnceLock::new();

#[cfg(any(feature = "client", feature = "ssr"))]
pub(crate) fn global_bridge_install_lock() -> MutexGuard<'static, ()> {
    GLOBAL_BRIDGE_INSTALL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(any(feature = "client", feature = "ssr"))]
fn installed_bridge() -> &'static RwLock<Option<InstalledBridge>> {
    INSTALLED_BRIDGE.get_or_init(|| RwLock::new(None))
}

#[cfg(any(feature = "client", feature = "ssr"))]
pub fn current_dioxus_global_localizer_owner() -> Option<DioxusGlobalLocalizerOwner> {
    installed_bridge()
        .read()
        .as_ref()
        .filter(|bridge| bridge.is_current_custom_localizer())
        .map(InstalledBridge::owner)
}

#[cfg(not(any(feature = "client", feature = "ssr")))]
pub fn current_dioxus_global_localizer_owner() -> Option<DioxusGlobalLocalizerOwner> {
    None
}

pub fn is_dioxus_bridge_current() -> bool {
    current_dioxus_global_localizer_owner().is_some()
}

#[cfg(any(feature = "client", feature = "ssr"))]
fn reject_stale_bridge(bridge: &Option<InstalledBridge>) -> Result<(), DioxusGlobalLocalizerError> {
    if let Some(active) = bridge.as_ref()
        && !active.is_current_custom_localizer()
    {
        return Err(DioxusGlobalLocalizerError::external_replacement(
            active.owner(),
        ));
    }

    Ok(())
}

#[cfg(all(test, any(feature = "client", feature = "ssr")))]
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
    reject_stale_bridge(bridge)?;

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
    let localizer =
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            localize_with_manager(&callback_manager, domain, id, args)
        };
    let generation = try_set_custom_localizer_lookup_with_domain_and_generation(localizer)?;

    *bridge = Some(InstalledBridge::Client {
        manager,
        generation,
    });
    Ok(generation)
}

#[cfg(feature = "ssr")]
pub(crate) fn install_ssr_bridge() -> Result<(), DioxusGlobalLocalizerError> {
    let _guard = global_bridge_install_lock();
    let mut bridge = installed_bridge().write();
    install_ssr_bridge_locked(&mut bridge).map(|_| ())
}

#[cfg(feature = "ssr")]
fn install_ssr_bridge_locked(
    bridge: &mut Option<InstalledBridge>,
) -> Result<CustomLocalizerGeneration, DioxusGlobalLocalizerError> {
    reject_stale_bridge(bridge)?;

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

    let generation = try_set_custom_localizer_lookup_with_domain_and_generation(
        crate::ssr::localize_current_ssr_manager,
    )?;
    *bridge = Some(InstalledBridge::Ssr { generation });
    Ok(generation)
}

#[cfg(feature = "client")]
fn localize_with_manager<'a>(
    manager: &FluentManager,
    domain: Option<&str>,
    id: &str,
    args: Option<&HashMap<&str, FluentValue<'a>>>,
) -> CustomLocalizerLookup {
    let message = match domain {
        Some(domain) => manager.localize_in_domain(domain, id, args),
        None => manager.localize(id, args),
    };

    match message {
        Some(message) => CustomLocalizerLookup::Found(message),
        None => {
            match domain {
                Some(domain) => tracing::warn!(domain, message_id = id, "missing Fluent message"),
                None => tracing::warn!(message_id = id, "missing Fluent message"),
            }
            CustomLocalizerLookup::Missing
        },
    }
}
