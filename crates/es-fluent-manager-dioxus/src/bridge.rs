use es_fluent::{
    CustomLocalizerGeneration, CustomLocalizerSnapshot, FluentValue, GlobalLocalizationError,
    current_custom_localizer_generation, custom_localizer_snapshot,
    replace_custom_localizer_with_domain_and_generation, restore_custom_localizer_snapshot,
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

#[derive(Clone)]
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

pub struct DioxusGlobalBridgeGuard {
    generation: Option<CustomLocalizerGeneration>,
    previous_bridge: Option<InstalledBridge>,
    previous_localizer: Option<CustomLocalizerSnapshot>,
}

impl DioxusGlobalBridgeGuard {
    fn inactive() -> Self {
        Self {
            generation: None,
            previous_bridge: None,
            previous_localizer: None,
        }
    }

    fn new(
        previous_localizer: CustomLocalizerSnapshot,
        previous_bridge: Option<InstalledBridge>,
        generation: CustomLocalizerGeneration,
    ) -> Self {
        Self {
            generation: Some(generation),
            previous_bridge,
            previous_localizer: Some(previous_localizer),
        }
    }
}

impl Drop for DioxusGlobalBridgeGuard {
    fn drop(&mut self) {
        let Some(generation) = self.generation else {
            return;
        };

        let _guard = global_bridge_install_lock();
        let mut bridge = installed_bridge().write();

        if current_custom_localizer_generation() == Some(generation) {
            if let Some(previous_localizer) = self.previous_localizer.take() {
                restore_custom_localizer_snapshot(previous_localizer);
            }
            *bridge = self.previous_bridge.take();
        } else if bridge
            .as_ref()
            .is_some_and(|bridge| bridge.generation() == generation)
        {
            *bridge = None;
        }
    }
}

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
    let mut bridge = installed_bridge().write();
    install_client_bridge_locked(&mut bridge, manager, policy).map(|_| ())
}

pub(crate) fn install_client_bridge_scoped(
    manager: Arc<FluentManager>,
    policy: GlobalBridgePolicy,
) -> Result<DioxusGlobalBridgeGuard, DioxusGlobalLocalizerError> {
    if policy == GlobalBridgePolicy::Disabled {
        return Ok(DioxusGlobalBridgeGuard::inactive());
    }

    let _guard = global_bridge_install_lock();
    let mut bridge = installed_bridge().write();

    discard_stale_bridge(&mut bridge);
    let previous_bridge = bridge.clone();
    let previous_localizer = custom_localizer_snapshot();

    let generation = install_client_bridge_locked(&mut bridge, manager, policy)?;
    Ok(DioxusGlobalBridgeGuard::new(
        previous_localizer,
        previous_bridge,
        generation,
    ))
}

fn install_client_bridge_locked(
    bridge: &mut Option<InstalledBridge>,
    manager: Arc<FluentManager>,
    policy: GlobalBridgePolicy,
) -> Result<CustomLocalizerGeneration, DioxusGlobalLocalizerError> {
    let requested_owner = BridgeOwner::Client(Arc::as_ptr(&manager) as usize);

    discard_stale_bridge(bridge);

    match bridge.as_ref().map(InstalledBridge::owner) {
        Some(active_owner)
            if active_owner == requested_owner && policy == GlobalBridgePolicy::InstallOnce =>
        {
            return Ok(bridge
                .as_ref()
                .expect("active bridge should exist")
                .generation());
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
    Ok(generation)
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
    let mut bridge = installed_bridge().write();
    install_ssr_bridge_locked(&mut bridge, policy, callback).map(|_| ())
}

#[cfg(feature = "ssr")]
pub(crate) fn install_ssr_bridge_scoped<F>(
    policy: GlobalBridgePolicy,
    callback: F,
) -> Result<DioxusGlobalBridgeGuard, DioxusGlobalLocalizerError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    if policy == GlobalBridgePolicy::Disabled {
        return Ok(DioxusGlobalBridgeGuard::inactive());
    }

    let _guard = global_bridge_install_lock();
    let mut bridge = installed_bridge().write();

    discard_stale_bridge(&mut bridge);
    let previous_bridge = bridge.clone();
    let previous_localizer = custom_localizer_snapshot();

    let generation = install_ssr_bridge_locked(&mut bridge, policy, callback)?;
    Ok(DioxusGlobalBridgeGuard::new(
        previous_localizer,
        previous_bridge,
        generation,
    ))
}

#[cfg(feature = "ssr")]
fn install_ssr_bridge_locked<F>(
    bridge: &mut Option<InstalledBridge>,
    policy: GlobalBridgePolicy,
    callback: F,
) -> Result<CustomLocalizerGeneration, DioxusGlobalLocalizerError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    let requested_owner = BridgeOwner::Ssr;

    discard_stale_bridge(bridge);

    match bridge.as_ref().map(InstalledBridge::owner) {
        Some(active_owner)
            if active_owner == requested_owner && policy == GlobalBridgePolicy::InstallOnce =>
        {
            return Ok(bridge
                .as_ref()
                .expect("active bridge should exist")
                .generation());
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
    Ok(generation)
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
