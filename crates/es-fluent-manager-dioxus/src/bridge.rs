use es_fluent::GlobalLocalizationError;
use parking_lot::RwLock;
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

static GLOBAL_BRIDGE_INSTALL_LOCK: Mutex<()> = Mutex::new(());
static ACTIVE_BRIDGE_OWNER: OnceLock<RwLock<Option<BridgeOwner>>> = OnceLock::new();

pub(crate) fn global_bridge_install_lock() -> MutexGuard<'static, ()> {
    GLOBAL_BRIDGE_INSTALL_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn active_bridge_owner() -> &'static RwLock<Option<BridgeOwner>> {
    ACTIVE_BRIDGE_OWNER.get_or_init(|| RwLock::new(None))
}
