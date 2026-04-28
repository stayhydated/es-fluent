#![doc = include_str!("../README.md")]

use es_fluent::{FluentLocalizer, FluentLocalizerExt, FluentMessage, FluentValue};
use es_fluent_manager_core::{FluentManager, LocalizationError, ModuleDiscoveryError};
use std::collections::HashMap;
use std::sync::Arc;
use unic_langid::LanguageIdentifier;

#[doc(hidden)]
pub use es_fluent::__inventory;

#[doc(hidden)]
pub use es_fluent::__rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_embedded_i18n_module as define_i18n_module;

pub use es_fluent_manager_core::LocalizationError as LanguageSelectionError;

#[derive(Debug)]
pub enum EmbeddedInitError {
    ModuleDiscovery(Vec<ModuleDiscoveryError>),
    LanguageSelection(LocalizationError),
}

impl std::fmt::Display for EmbeddedInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDiscovery(errors) => {
                f.write_str("failed strict i18n module discovery")?;
                for error in errors {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            },
            Self::LanguageSelection(error) => {
                write!(f, "failed to select the requested language: {error}")
            },
        }
    }
}

impl std::error::Error for EmbeddedInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection(error) => Some(error),
        }
    }
}

/// Explicit embedded localization context.
///
/// Construct this once during application startup, keep it in application state,
/// and pass it to code that needs localization. No context-free `es-fluent`
/// context is installed.
#[derive(Clone)]
pub struct EmbeddedI18n {
    manager: Arc<FluentManager>,
}

impl EmbeddedI18n {
    fn from_manager(manager: FluentManager) -> Self {
        Self {
            manager: Arc::new(manager),
        }
    }

    /// Builds an embedded context without selecting a language.
    pub fn try_new() -> Result<Self, EmbeddedInitError> {
        FluentManager::try_new_with_discovered_modules()
            .map(Self::from_manager)
            .map_err(EmbeddedInitError::ModuleDiscovery)
    }

    /// Builds an embedded context and selects the initial active language.
    pub fn try_new_with_language<L: Into<LanguageIdentifier>>(
        lang: L,
    ) -> Result<Self, EmbeddedInitError> {
        let lang = lang.into();
        let manager = FluentManager::try_new_with_discovered_modules()
            .map_err(EmbeddedInitError::ModuleDiscovery)?;
        manager
            .select_language(&lang)
            .map_err(EmbeddedInitError::LanguageSelection)?;
        Ok(Self::from_manager(manager))
    }

    /// Returns the underlying manager.
    pub fn manager(&self) -> &FluentManager {
        &self.manager
    }

    /// Selects the active language for this context.
    pub fn select_language<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.manager.select_language(&lang.into())
    }

    /// Selects the active language for this context and fails if any runtime
    /// module rejects the requested locale.
    pub fn select_language_strict<L: Into<LanguageIdentifier>>(
        &self,
        lang: L,
    ) -> Result<(), LocalizationError> {
        self.manager.select_language_strict(&lang.into())
    }

    /// Localizes a message by ID using manager discovery order.
    pub fn localize<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize(id.as_ref(), args)
    }

    /// Localizes a message by ID, falling back to the ID on misses.
    pub fn localize_or_id<'a>(
        &self,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        FluentLocalizerExt::localize_or_id(self, id.as_ref(), args)
    }

    /// Localizes a message by ID within a specific domain.
    pub fn localize_in_domain<'a>(
        &self,
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager
            .localize_in_domain(domain.as_ref(), id.as_ref(), args)
    }

    /// Localizes a domain-scoped message by ID, falling back to the ID on
    /// misses.
    pub fn localize_in_domain_or_id<'a>(
        &self,
        domain: impl AsRef<str>,
        id: impl AsRef<str>,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> String {
        FluentLocalizerExt::localize_in_domain_or_id(self, domain.as_ref(), id.as_ref(), args)
    }

    /// Renders a derived typed message through this context.
    pub fn localize_message<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message(self, message)
    }

    /// Renders a derived typed message through this context without logging
    /// missing-message warnings.
    pub fn localize_message_silent<T>(&self, message: &T) -> String
    where
        T: FluentMessage + ?Sized,
    {
        FluentLocalizerExt::localize_message_silent(self, message)
    }
}

impl FluentLocalizer for EmbeddedI18n {
    fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize(id, args)
    }

    fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.manager.localize_in_domain(domain, id, args)
    }
}
