use es_fluent::{
    CustomLocalizerGeneration, FluentValue, GlobalLocalizationError,
    current_custom_localizer_generation, replace_custom_localizer_with_domain_and_generation,
    try_set_custom_localizer_with_domain_and_generation,
};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GlobalBridgePolicy {
    Disabled,
    #[default]
    InstallOnce,
    ReplaceExisting,
}

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
        policy: GlobalBridgePolicy,
    },
    Global(Arc<GlobalLocalizationError>),
}

impl DioxusGlobalLocalizerError {
    pub(crate) fn owner_conflict(
        active: BridgeOwner,
        requested: BridgeOwner,
        policy: GlobalBridgePolicy,
    ) -> Self {
        Self::OwnerConflict {
            active: active.public_owner(),
            requested: requested.public_owner(),
            policy,
        }
    }
}

impl std::fmt::Display for DioxusGlobalLocalizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Client,
                requested: DioxusGlobalLocalizerOwner::Client,
                policy,
            } => write!(
                f,
                "a different Dioxus client manager already owns the global Fluent localizer; \
                 requested the client bridge with {policy:?}",
            ),
            Self::OwnerConflict {
                active,
                requested,
                policy,
            } => write!(
                f,
                "the Dioxus global Fluent localizer is already owned by the {active} bridge; \
                 requested the {requested} bridge with {policy:?}",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BridgeOwner {
    Client(usize),
    #[cfg(feature = "ssr")]
    Ssr,
}

impl BridgeOwner {
    pub(crate) fn public_owner(self) -> DioxusGlobalLocalizerOwner {
        match self {
            Self::Client(_) => DioxusGlobalLocalizerOwner::Client,
            #[cfg(feature = "ssr")]
            Self::Ssr => DioxusGlobalLocalizerOwner::Ssr,
        }
    }
}

enum InstalledBridge {
    Client {
        owner_id: usize,
        manager: Arc<FluentManager>,
        generation: CustomLocalizerGeneration,
    },
    #[cfg(feature = "ssr")]
    Ssr {
        generation: CustomLocalizerGeneration,
    },
}

impl InstalledBridge {
    fn owner(&self) -> BridgeOwner {
        match self {
            Self::Client { owner_id, .. } => BridgeOwner::Client(*owner_id),
            #[cfg(feature = "ssr")]
            Self::Ssr { .. } => BridgeOwner::Ssr,
        }
    }

    fn generation(&self) -> CustomLocalizerGeneration {
        match self {
            Self::Client { generation, .. } => *generation,
            #[cfg(feature = "ssr")]
            Self::Ssr { generation } => *generation,
        }
    }

    fn is_current_custom_localizer(&self) -> bool {
        current_custom_localizer_generation() == Some(self.generation())
    }
}

static GLOBAL_BRIDGE_INSTALL_LOCK: Mutex<()> = Mutex::new(());
static INSTALLED_BRIDGE: OnceLock<RwLock<Option<InstalledBridge>>> = OnceLock::new();

pub(crate) fn global_bridge_install_lock() -> MutexGuard<'static, ()> {
    GLOBAL_BRIDGE_INSTALL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn installed_bridge() -> &'static RwLock<Option<InstalledBridge>> {
    INSTALLED_BRIDGE.get_or_init(|| RwLock::new(None))
}

pub(crate) fn current_client_bridge_manager() -> Option<Arc<FluentManager>> {
    match installed_bridge().read().as_ref() {
        Some(bridge @ InstalledBridge::Client { manager, .. })
            if bridge.is_current_custom_localizer() =>
        {
            Some(Arc::clone(manager))
        },
        Some(InstalledBridge::Client { .. }) => None,
        #[cfg(feature = "ssr")]
        Some(InstalledBridge::Ssr { .. }) | None => None,
        #[cfg(not(feature = "ssr"))]
        None => None,
    }
}

pub(crate) fn install_client_bridge(
    manager: Arc<FluentManager>,
    policy: GlobalBridgePolicy,
) -> Result<(), DioxusGlobalLocalizerError> {
    if policy == GlobalBridgePolicy::Disabled {
        return Ok(());
    }

    let _guard = global_bridge_install_lock();
    let requested_owner = BridgeOwner::Client(Arc::as_ptr(&manager) as usize);
    let mut bridge = installed_bridge().write();

    if bridge
        .as_ref()
        .is_some_and(|bridge| !bridge.is_current_custom_localizer())
    {
        *bridge = None;
    }

    match bridge.as_ref().map(InstalledBridge::owner) {
        Some(active_owner)
            if active_owner == requested_owner && policy == GlobalBridgePolicy::InstallOnce =>
        {
            return Ok(());
        },
        Some(active_owner) if policy == GlobalBridgePolicy::InstallOnce => {
            return Err(DioxusGlobalLocalizerError::owner_conflict(
                active_owner,
                requested_owner,
                policy,
            ));
        },
        _ => {},
    }

    let generation = install_client_bridge_callback(policy)?;
    *bridge = Some(InstalledBridge::Client {
        owner_id: Arc::as_ptr(&manager) as usize,
        manager,
        generation,
    });
    Ok(())
}

#[cfg(feature = "ssr")]
pub(crate) fn install_ssr_bridge<F>(
    policy: GlobalBridgePolicy,
    callback: F,
) -> Result<(), DioxusGlobalLocalizerError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    if policy == GlobalBridgePolicy::Disabled {
        return Ok(());
    }

    let _guard = global_bridge_install_lock();
    let requested_owner = BridgeOwner::Ssr;
    let mut bridge = installed_bridge().write();

    if bridge
        .as_ref()
        .is_some_and(|bridge| !bridge.is_current_custom_localizer())
    {
        *bridge = None;
    }

    match bridge.as_ref().map(InstalledBridge::owner) {
        Some(active_owner)
            if active_owner == requested_owner && policy == GlobalBridgePolicy::InstallOnce =>
        {
            return Ok(());
        },
        Some(active_owner) if policy == GlobalBridgePolicy::InstallOnce => {
            return Err(DioxusGlobalLocalizerError::owner_conflict(
                active_owner,
                requested_owner,
                policy,
            ));
        },
        _ => {},
    }

    let generation = install_custom_localizer(policy, callback)?;
    *bridge = Some(InstalledBridge::Ssr { generation });
    Ok(())
}

fn install_client_bridge_callback(
    policy: GlobalBridgePolicy,
) -> Result<CustomLocalizerGeneration, GlobalLocalizationError> {
    install_custom_localizer(
        policy,
        move |domain: Option<&str>, id: &str, args: Option<&HashMap<&str, FluentValue<'_>>>| {
            let manager = current_client_bridge_manager()?;
            let message = match domain {
                Some(domain) => manager.localize_in_domain(domain, id, args),
                None => manager.localize(id, args),
            };

            match message {
                Some(message) => Some(message),
                None => {
                    match domain {
                        Some(domain) => {
                            tracing::warn!(domain, message_id = id, "missing Fluent message");
                        },
                        None => {
                            tracing::warn!(message_id = id, "missing Fluent message");
                        },
                    }
                    Some(id.to_string())
                },
            }
        },
    )
}

pub(crate) fn install_custom_localizer<F>(
    policy: GlobalBridgePolicy,
    callback: F,
) -> Result<CustomLocalizerGeneration, GlobalLocalizationError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    match policy {
        GlobalBridgePolicy::Disabled => {
            unreachable!("disabled global bridge policy is handled before installation")
        },
        GlobalBridgePolicy::InstallOnce => {
            try_set_custom_localizer_with_domain_and_generation(callback)
        },
        GlobalBridgePolicy::ReplaceExisting => {
            tracing::debug!("replacing the process-global Fluent custom localizer with Dioxus");
            Ok(replace_custom_localizer_with_domain_and_generation(
                callback,
            ))
        },
    }
}
