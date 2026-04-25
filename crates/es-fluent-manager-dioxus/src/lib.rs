#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub use es_fluent::__inventory;

#[doc(hidden)]
pub use es_fluent::__rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

pub use es_fluent::GlobalLocalizationError;

mod bridge;
mod error;
mod managed;

#[cfg(any(feature = "desktop", feature = "mobile", feature = "web"))]
mod client;

#[cfg(feature = "ssr")]
pub mod ssr;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_dioxus_i18n_module as define_i18n_module;

pub(crate) use bridge::{BridgeOwner, active_bridge_owner, global_bridge_install_lock};
pub use bridge::{DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, GlobalLocalizerMode};
pub use error::DioxusInitError;
pub use managed::ManagedI18n;

#[cfg(any(feature = "desktop", feature = "mobile", feature = "web"))]
pub use client::{
    DioxusI18n, use_i18n, use_init_i18n, use_init_i18n_with_mode, use_localized, use_provide_i18n,
    use_provide_i18n_with_mode, use_try_init_i18n, use_try_init_i18n_with_mode,
    use_try_provide_i18n, use_try_provide_i18n_with_mode,
};

#[cfg(feature = "desktop")]
pub mod desktop {
    pub use crate::{
        DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusI18n, DioxusInitError,
        GlobalLocalizerMode, ManagedI18n, use_i18n, use_init_i18n, use_init_i18n_with_mode,
        use_localized, use_provide_i18n, use_provide_i18n_with_mode, use_try_init_i18n,
        use_try_init_i18n_with_mode, use_try_provide_i18n, use_try_provide_i18n_with_mode,
    };
}

#[cfg(feature = "mobile")]
pub mod mobile {
    pub use crate::{
        DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusI18n, DioxusInitError,
        GlobalLocalizerMode, ManagedI18n, use_i18n, use_init_i18n, use_init_i18n_with_mode,
        use_localized, use_provide_i18n, use_provide_i18n_with_mode, use_try_init_i18n,
        use_try_init_i18n_with_mode, use_try_provide_i18n, use_try_provide_i18n_with_mode,
    };
}

#[cfg(feature = "web")]
pub mod web {
    pub use crate::{
        DioxusGlobalLocalizerError, DioxusGlobalLocalizerOwner, DioxusI18n, DioxusInitError,
        GlobalLocalizerMode, ManagedI18n, use_i18n, use_init_i18n, use_init_i18n_with_mode,
        use_localized, use_provide_i18n, use_provide_i18n_with_mode, use_try_init_i18n,
        use_try_init_i18n_with_mode, use_try_provide_i18n, use_try_provide_i18n_with_mode,
    };
}

#[cfg(test)]
mod tests;
