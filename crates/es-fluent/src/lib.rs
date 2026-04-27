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
use parking_lot::RwLock;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU64, Ordering},
};

#[cfg(feature = "build")]
pub mod build {
    pub use es_fluent_toml::build::*;
}

mod traits;
pub use traits::{EsFluentChoice, FluentDisplay, FluentMessage, ThisFtl, ToFluentString};

#[doc(hidden)]
pub mod __private {
    pub use crate::traits::{
        FluentArgumentValue, FluentMessageStringValue, IntoFluentArgumentValue,
        IntoFluentMessageString,
    };
}

#[doc(hidden)]
static CONTEXT: OnceLock<ArcSwap<FluentManager>> = OnceLock::new();

#[doc(hidden)]
type DomainAwareCustomLocalizer = dyn for<'a> Fn(
        Option<&str>,
        &str,
        Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
    ) -> CustomLocalizerLookup
    + Send
    + Sync;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CustomLocalizerGeneration(u64);

#[doc(hidden)]
pub enum CustomLocalizerLookup {
    Continue,
    Missing,
    Found(String),
}

impl CustomLocalizerLookup {
    fn from_optional(message: Option<String>) -> Self {
        match message {
            Some(message) => Self::Found(message),
            None => Self::Continue,
        }
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct CustomLocalizerSnapshot {
    entry: Option<Arc<DomainAwareCustomLocalizerEntry>>,
}

struct DomainAwareCustomLocalizerEntry {
    generation: CustomLocalizerGeneration,
    localizer: Arc<DomainAwareCustomLocalizer>,
}

#[doc(hidden)]
static CUSTOM_LOCALIZER: OnceLock<RwLock<Option<Arc<DomainAwareCustomLocalizerEntry>>>> =
    OnceLock::new();

static CUSTOM_LOCALIZER_GENERATION: AtomicU64 = AtomicU64::new(1);

fn next_custom_localizer_generation() -> CustomLocalizerGeneration {
    CustomLocalizerGeneration(CUSTOM_LOCALIZER_GENERATION.fetch_add(1, Ordering::Relaxed))
}

fn custom_localizer_slot() -> &'static RwLock<Option<Arc<DomainAwareCustomLocalizerEntry>>> {
    CUSTOM_LOCALIZER.get_or_init(|| RwLock::new(None))
}

fn try_custom_localizer<'a>(
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> CustomLocalizerLookup {
    CUSTOM_LOCALIZER
        .get()
        .and_then(|slot| slot.read().clone())
        .map_or(CustomLocalizerLookup::Continue, |entry| {
            (entry.localizer)(None, id, args)
        })
}

fn try_custom_localizer_in_domain<'a>(
    domain: &str,
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> CustomLocalizerLookup {
    CUSTOM_LOCALIZER
        .get()
        .and_then(|slot| slot.read().clone())
        .map_or(CustomLocalizerLookup::Continue, |entry| {
            (entry.localizer)(Some(domain), id, args)
        })
}

#[doc(hidden)]
pub fn current_custom_localizer_generation() -> Option<CustomLocalizerGeneration> {
    CUSTOM_LOCALIZER
        .get()
        .and_then(|slot| slot.read().as_ref().map(|entry| entry.generation))
}

#[doc(hidden)]
pub fn custom_localizer_snapshot() -> CustomLocalizerSnapshot {
    CustomLocalizerSnapshot {
        entry: CUSTOM_LOCALIZER
            .get()
            .and_then(|slot| slot.read().as_ref().cloned()),
    }
}

#[doc(hidden)]
pub fn restore_custom_localizer_snapshot(snapshot: CustomLocalizerSnapshot) {
    *custom_localizer_slot().write() = snapshot.entry;
}

#[doc(hidden)]
pub fn clear_custom_localizer_generation(generation: CustomLocalizerGeneration) -> bool {
    let mut slot = custom_localizer_slot().write();
    if slot
        .as_ref()
        .is_some_and(|entry| entry.generation == generation)
    {
        *slot = None;
        true
    } else {
        false
    }
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

/// Sets a domain-aware custom localizer function.
///
/// The callback receives `None` for plain [`localize`] requests and
/// `Some(domain)` for [`localize_in_domain`] requests.
#[doc(hidden)]
pub fn set_custom_localizer_with_domain<F>(localizer: F)
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    try_set_custom_localizer_with_domain(localizer)
        .expect("Failed to set domain-aware custom localizer");
}

#[doc(hidden)]
pub fn try_set_custom_localizer_with_domain<F>(localizer: F) -> Result<(), GlobalLocalizationError>
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    try_set_custom_localizer_with_domain_and_generation(localizer).map(|_| ())
}

#[doc(hidden)]
pub fn try_set_custom_localizer_with_domain_and_generation<F>(
    localizer: F,
) -> Result<CustomLocalizerGeneration, GlobalLocalizationError>
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    try_set_custom_localizer_lookup_with_domain_and_generation(move |domain, id, args| {
        CustomLocalizerLookup::from_optional(localizer(domain, id, args))
    })
}

#[doc(hidden)]
pub fn try_set_custom_localizer_lookup_with_domain_and_generation<F>(
    localizer: F,
) -> Result<CustomLocalizerGeneration, GlobalLocalizationError>
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> CustomLocalizerLookup
        + Send
        + Sync
        + 'static,
{
    let mut slot = custom_localizer_slot().write();
    if slot.is_some() {
        return Err(GlobalLocalizationError::CustomLocalizerAlreadyInitialized);
    }
    let generation = next_custom_localizer_generation();
    *slot = Some(Arc::new(DomainAwareCustomLocalizerEntry {
        generation,
        localizer: Arc::new(localizer),
    }));
    Ok(generation)
}

/// Replaces the custom localizer with a domain-aware callback.
#[doc(hidden)]
pub fn replace_custom_localizer_with_domain<F>(localizer: F)
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    replace_custom_localizer_with_domain_and_generation(localizer);
}

#[doc(hidden)]
pub fn replace_custom_localizer_with_domain_and_generation<F>(
    localizer: F,
) -> CustomLocalizerGeneration
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> Option<String>
        + Send
        + Sync
        + 'static,
{
    replace_custom_localizer_lookup_with_domain_and_generation(move |domain, id, args| {
        CustomLocalizerLookup::from_optional(localizer(domain, id, args))
    })
}

#[doc(hidden)]
pub fn replace_custom_localizer_lookup_with_domain_and_generation<F>(
    localizer: F,
) -> CustomLocalizerGeneration
where
    F: for<'a> Fn(
            Option<&str>,
            &str,
            Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
        ) -> CustomLocalizerLookup
        + Send
        + Sync
        + 'static,
{
    let generation = next_custom_localizer_generation();
    *custom_localizer_slot().write() = Some(Arc::new(DomainAwareCustomLocalizerEntry {
        generation,
        localizer: Arc::new(localizer),
    }));
    generation
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
    match try_custom_localizer(id, args) {
        CustomLocalizerLookup::Found(message) => return message,
        CustomLocalizerLookup::Missing => {
            tracing::warn!("Translation for '{}' not found or context not set.", id);
            return id.to_string();
        },
        CustomLocalizerLookup::Continue => {},
    }

    if let Some(context) = CONTEXT.get()
        && let Some(message) = context.load().localize(id, args)
    {
        return message;
    }

    tracing::warn!("Translation for '{}' not found or context not set.", id);
    id.to_string()
}

/// Localizes a message by its ID within the given domain.
///
/// This first consults any installed custom localizer, then performs an
/// explicit domain-scoped lookup against the shared context. Custom
/// localizers receive `Some(domain)` for domain-scoped requests.
#[doc(hidden)]
pub fn localize_in_domain<'a>(
    domain: &str,
    id: &str,
    args: Option<&std::collections::HashMap<&str, FluentValue<'a>>>,
) -> String {
    match try_custom_localizer_in_domain(domain, id, args) {
        CustomLocalizerLookup::Found(message) => return message,
        CustomLocalizerLookup::Missing => {
            tracing::warn!(
                "Translation for '{}' in domain '{}' not found or context not set.",
                id,
                domain
            );
            return id.to_string();
        },
        CustomLocalizerLookup::Continue => {},
    }

    if let Some(context) = CONTEXT.get()
        && let Some(message) = context.load().localize_in_domain(domain, id, args)
    {
        return message;
    }

    tracing::warn!(
        "Translation for '{}' in domain '{}' not found or context not set.",
        id,
        domain
    );
    id.to_string()
}
