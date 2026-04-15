#![doc = include_str!("../README.md")]

#[doc(hidden)]
pub mod meta;

#[doc(hidden)]
pub mod registry;

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentThis, EsFluentVariants};

#[doc(hidden)]
pub use fluent_bundle::FluentValue;

#[doc(hidden)]
pub use inventory as __inventory;

#[doc(hidden)]
pub use rust_embed as __rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid;

use arc_swap::ArcSwap;
use es_fluent_manager_core::FluentManager;
use es_fluent_shared::EsFluentError;
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(feature = "build")]
pub mod build {
    pub use es_fluent_toml::build::*;
}

mod traits;
pub use traits::{EsFluentChoice, FluentDisplay, ThisFtl, ToFluentString};

#[doc(hidden)]
static CONTEXT: OnceLock<ArcSwap<FluentManager>> = OnceLock::new();

#[doc(hidden)]
type CustomLocalizer = dyn for<'a> Fn(&str, Option<&std::collections::HashMap<&str, FluentValue<'a>>>) -> Option<String>
    + Send
    + Sync;

#[doc(hidden)]
static CUSTOM_LOCALIZER: OnceLock<RwLock<Option<Arc<CustomLocalizer>>>> = OnceLock::new();

fn custom_localizer_slot() -> &'static RwLock<Option<Arc<CustomLocalizer>>> {
    CUSTOM_LOCALIZER.get_or_init(|| RwLock::new(None))
}

#[derive(Debug)]
pub enum GlobalLocalizationError {
    ContextAlreadyInitialized,
    ContextNotInitialized,
    CustomLocalizerAlreadyInitialized,
    LanguageSelectionFailed(EsFluentError),
}

impl std::fmt::Display for GlobalLocalizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContextAlreadyInitialized => {
                f.write_str("the global FluentManager context is already initialized")
            },
            Self::ContextNotInitialized => {
                f.write_str("the global FluentManager context is not initialized")
            },
            Self::CustomLocalizerAlreadyInitialized => {
                f.write_str("the global custom localizer is already initialized")
            },
            Self::LanguageSelectionFailed(source) => {
                write!(f, "failed to select the requested language: {source}")
            },
        }
    }
}

impl std::error::Error for GlobalLocalizationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LanguageSelectionFailed(source) => Some(source),
            _ => None,
        }
    }
}

impl From<EsFluentError> for GlobalLocalizationError {
    fn from(value: EsFluentError) -> Self {
        Self::LanguageSelectionFailed(value)
    }
}

/// Sets the global `FluentManager` context.
///
/// This function should be called once at the beginning of your application's
/// lifecycle.
///
/// # Panics
///
/// This function will panic if the context has already been set.
#[doc(hidden)]
pub fn set_context(manager: FluentManager) {
    try_set_context(manager).expect("Failed to set context");
}

/// Sets the global `FluentManager` context with a shared `ArcSwap<FluentManager>`.
///
/// This function is useful when you want to share the `FluentManager` between
/// multiple threads.
///
/// # Panics
///
/// This function will panic if the context has already been set.
#[doc(hidden)]
pub fn set_shared_context(manager: Arc<FluentManager>) {
    try_set_shared_context(manager).expect("Failed to set shared context");
}

#[doc(hidden)]
pub fn try_set_context(manager: FluentManager) -> Result<(), GlobalLocalizationError> {
    CONTEXT
        .set(ArcSwap::from_pointee(manager))
        .map_err(|_| GlobalLocalizationError::ContextAlreadyInitialized)
}

#[doc(hidden)]
pub fn try_set_shared_context(manager: Arc<FluentManager>) -> Result<(), GlobalLocalizationError> {
    CONTEXT
        .set(ArcSwap::new(manager))
        .map_err(|_| GlobalLocalizationError::ContextAlreadyInitialized)
}

/// Sets a custom localizer function.
///
/// The custom localizer will be called before the global context's `localize`
/// method. If the custom localizer returns `Some(message)`, the message will be
/// returned. Otherwise, the global context will be used.
///
/// # Panics
///
/// This function will panic if the custom localizer has already been set.
#[doc(hidden)]
pub fn set_custom_localizer<F>(localizer: F)
where
    F: for<'a> Fn(
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    try_set_custom_localizer(localizer).expect("Failed to set custom localizer");
}

#[doc(hidden)]
pub fn try_set_custom_localizer<F>(localizer: F) -> Result<(), GlobalLocalizationError>
where
    F: for<'a> Fn(
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    let mut slot = custom_localizer_slot()
        .write()
        .expect("custom localizer lock poisoned");
    if slot.is_some() {
        return Err(GlobalLocalizationError::CustomLocalizerAlreadyInitialized);
    }
    *slot = Some(Arc::new(localizer));
    Ok(())
}

/// Replaces the custom localizer function.
///
/// This is intended for integrations that deliberately own the process-global
/// localization hook and need to refresh or reinstall it.
#[doc(hidden)]
pub fn replace_custom_localizer<F>(localizer: F)
where
    F: for<'a> Fn(
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    *custom_localizer_slot()
        .write()
        .expect("custom localizer lock poisoned") = Some(Arc::new(localizer));
}

/// Selects a language for all localizers in the global context.
#[doc(hidden)]
pub fn select_language(
    lang: &unic_langid::LanguageIdentifier,
) -> Result<(), GlobalLocalizationError> {
    let context = CONTEXT
        .get()
        .ok_or(GlobalLocalizationError::ContextNotInitialized)?;
    context.load().select_language(lang).map_err(Into::into)
}

/// Localizes a message by its ID.
///
/// This function will first try to use the custom localizer if it has been set.
/// If the custom localizer returns `None`, it will then try to use the global
/// context.
///
/// If the message is not found, a warning will be logged and the ID will be
/// returned as the message.
#[doc(hidden)]
pub fn localize<'a>(
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> String {
    if let Some(custom_localizer) = CUSTOM_LOCALIZER.get().and_then(|custom_localizer| {
        custom_localizer
            .read()
            .expect("custom localizer lock poisoned")
            .clone()
    }) && let Some(message) = custom_localizer(id, args)
    {
        return message;
    }

    if let Some(context) = CONTEXT.get()
        && let Some(message) = context.load().localize(id, args)
    {
        return message;
    }

    tracing::warn!("Translation for '{}' not found or context not set.", id);
    id.to_string()
}
