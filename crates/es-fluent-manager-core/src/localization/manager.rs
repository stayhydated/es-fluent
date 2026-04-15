use super::{
    I18nModuleRegistration, Localizer, ModuleDiscoveryError, filter_module_registry,
    try_filter_module_registry,
};
use crate::asset_localization::ModuleData;
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use unic_langid::LanguageIdentifier;

/// A manager for Fluent translations.
#[derive(Default)]
pub struct FluentManager {
    pub(super) localizers: Vec<(&'static ModuleData, Box<dyn Localizer>)>,
}

impl FluentManager {
    /// Creates a new `FluentManager` with discovered i18n modules.
    pub fn new_with_discovered_modules() -> Self {
        let discovered_modules = filter_module_registry(
            inventory::iter::<&'static dyn I18nModuleRegistration>()
                .copied()
                .collect::<Vec<_>>(),
        );

        let mut manager = Self::default();

        for module in discovered_modules {
            let data = module.data();
            tracing::info!("Discovered and loading i18n module: {}", data.name);
            if let Some(localizer) = module.create_localizer() {
                manager.localizers.push((data, localizer));
            } else {
                tracing::debug!(
                    "Skipping metadata-only i18n module '{}' for FluentManager runtime localization",
                    data.name
                );
            }
        }
        manager
    }

    /// Creates a new `FluentManager` with strict registry validation.
    ///
    /// Unlike [`Self::new_with_discovered_modules`], this returns an error when
    /// discovery finds invalid module metadata or unresolvable duplicate
    /// registrations.
    pub fn try_new_with_discovered_modules() -> Result<Self, Vec<ModuleDiscoveryError>> {
        let discovered_modules = try_filter_module_registry(
            inventory::iter::<&'static dyn I18nModuleRegistration>()
                .copied()
                .collect::<Vec<_>>(),
        )?;

        let mut manager = Self::default();

        for module in discovered_modules {
            let data = module.data();
            tracing::info!("Discovered and loading i18n module: {}", data.name);
            if let Some(localizer) = module.create_localizer() {
                manager.localizers.push((data, localizer));
            } else {
                tracing::debug!(
                    "Skipping metadata-only i18n module '{}' for FluentManager runtime localization",
                    data.name
                );
            }
        }

        Ok(manager)
    }

    /// Selects a language for all localizers.
    pub fn select_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> crate::localization::LocalizationErrorResult<()> {
        let mut any_selected = false;

        for (data, localizer) in &self.localizers {
            match localizer.select_language(lang) {
                Ok(()) => {
                    any_selected = true;
                },
                Err(e) => {
                    tracing::debug!(
                        "Module '{}' failed to set language '{}': {}",
                        data.name,
                        lang,
                        e
                    );
                },
            }
        }

        if !any_selected {
            tracing::warn!("No i18n modules support language '{}'", lang);
            return Err(crate::localization::LocalizationError::LanguageNotSupported(lang.clone()));
        }

        Ok(())
    }

    /// Localizes a message by its ID.
    ///
    /// This searches localizers in discovery order and returns the first match.
    /// Use [`Self::localize_in_domain`] when the caller needs domain-scoped
    /// lookup instead of first-match behavior.
    pub fn localize<'a>(
        &self,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        for (_, localizer) in &self.localizers {
            if let Some(message) = localizer.localize(id, args) {
                return Some(message);
            }
        }
        None
    }

    /// Localizes a message by its ID within a specific domain.
    pub fn localize_in_domain<'a>(
        &self,
        domain: &str,
        id: &str,
        args: Option<&HashMap<&str, FluentValue<'a>>>,
    ) -> Option<String> {
        self.localizers
            .iter()
            .find(|(data, _)| data.domain == domain)
            .and_then(|(_, localizer)| localizer.localize(id, args))
    }
}
