use es_fluent::{
    FluentValue, GlobalLocalizationError, replace_custom_localizer_with_domain,
    try_set_custom_localizer_with_domain,
};
use es_fluent_manager_core::FluentManager;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GlobalLocalizerMode {
    #[default]
    ErrorIfAlreadySet,
    ReplaceExisting,
    ReuseIfSameOwner,
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
        mode: GlobalLocalizerMode,
    },
    Global(Arc<GlobalLocalizationError>),
}

impl DioxusGlobalLocalizerError {
    pub(crate) fn owner_conflict(
        active: BridgeOwner,
        requested: BridgeOwner,
        mode: GlobalLocalizerMode,
    ) -> Self {
        Self::OwnerConflict {
            active: active.public_owner(),
            requested: requested.public_owner(),
            mode,
        }
    }
}

impl std::fmt::Display for DioxusGlobalLocalizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OwnerConflict {
                active: DioxusGlobalLocalizerOwner::Client,
                requested: DioxusGlobalLocalizerOwner::Client,
                mode,
            } => write!(
                f,
                "a different Dioxus client manager already owns the global Fluent localizer; \
                 requested the client bridge with {mode:?}",
            ),
            Self::OwnerConflict {
                active,
                requested,
                mode,
            } => write!(
                f,
                "the Dioxus global Fluent localizer is already owned by the {active} bridge; \
                 requested the {requested} bridge with {mode:?}",
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
    },
    #[cfg(feature = "ssr")]
    Ssr,
}

impl InstalledBridge {
    fn owner(&self) -> BridgeOwner {
        match self {
            Self::Client { owner_id, .. } => BridgeOwner::Client(*owner_id),
            #[cfg(feature = "ssr")]
            Self::Ssr => BridgeOwner::Ssr,
        }
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
        Some(InstalledBridge::Client { manager, .. }) => Some(Arc::clone(manager)),
        #[cfg(feature = "ssr")]
        Some(InstalledBridge::Ssr) | None => None,
        #[cfg(not(feature = "ssr"))]
        None => None,
    }
}

pub(crate) fn install_client_bridge(
    manager: Arc<FluentManager>,
    mode: GlobalLocalizerMode,
) -> Result<(), DioxusGlobalLocalizerError> {
    let _guard = global_bridge_install_lock();
    let requested_owner = BridgeOwner::Client(Arc::as_ptr(&manager) as usize);
    let mut bridge = installed_bridge().write();

    match bridge.as_ref().map(InstalledBridge::owner) {
        Some(active_owner)
            if active_owner == requested_owner && mode != GlobalLocalizerMode::ReplaceExisting =>
        {
            *bridge = Some(InstalledBridge::Client {
                owner_id: Arc::as_ptr(&manager) as usize,
                manager,
            });
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

    install_client_bridge_callback(mode)?;
    *bridge = Some(InstalledBridge::Client {
        owner_id: Arc::as_ptr(&manager) as usize,
        manager,
    });
    Ok(())
}

#[cfg(feature = "ssr")]
pub(crate) fn install_ssr_bridge<F>(
    mode: GlobalLocalizerMode,
    callback: F,
) -> Result<(), DioxusGlobalLocalizerError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    let _guard = global_bridge_install_lock();
    let requested_owner = BridgeOwner::Ssr;
    let mut bridge = installed_bridge().write();

    match bridge.as_ref().map(InstalledBridge::owner) {
        Some(active_owner)
            if active_owner == requested_owner && mode != GlobalLocalizerMode::ReplaceExisting =>
        {
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

    install_custom_localizer(mode, callback)?;
    *bridge = Some(InstalledBridge::Ssr);
    Ok(())
}

fn install_client_bridge_callback(
    mode: GlobalLocalizerMode,
) -> Result<(), GlobalLocalizationError> {
    install_custom_localizer(
        mode,
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
    mode: GlobalLocalizerMode,
    callback: F,
) -> Result<(), GlobalLocalizationError>
where
    F: for<'a> Fn(Option<&str>, &str, Option<&HashMap<&str, FluentValue<'a>>>) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    match mode {
        GlobalLocalizerMode::ErrorIfAlreadySet | GlobalLocalizerMode::ReuseIfSameOwner => {
            try_set_custom_localizer_with_domain(callback)
        },
        GlobalLocalizerMode::ReplaceExisting => {
            tracing::debug!("replacing the process-global Fluent custom localizer with Dioxus");
            replace_custom_localizer_with_domain(callback);
            Ok(())
        },
    }
}
