use super::{
    I18nModuleRegistration, LanguageSelectionPolicy, Localizer, ModuleDiscoveryError,
    ModuleRegistrationKind, filter_module_registry, try_filter_module_registry,
};
use crate::asset_localization::ModuleData;
use fluent_bundle::FluentValue;
use std::collections::HashMap;
use std::io;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use unic_langid::LanguageIdentifier;

type ManagedLocalizer = (&'static ModuleData, Box<dyn Localizer>);

/// A manager for Fluent translations.
#[derive(Default)]
pub struct FluentManager {
    pub(super) modules: Vec<&'static dyn I18nModuleRegistration>,
    pub(super) localizers: RwLock<Vec<ManagedLocalizer>>,
}

fn read_localizers(
    localizers: &RwLock<Vec<ManagedLocalizer>>,
) -> RwLockReadGuard<'_, Vec<ManagedLocalizer>> {
    match localizers.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("FluentManager localizer state lock poisoned while reading; recovering");
            poisoned.into_inner()
        },
    }
}

fn write_localizers(
    localizers: &RwLock<Vec<ManagedLocalizer>>,
) -> RwLockWriteGuard<'_, Vec<ManagedLocalizer>> {
    match localizers.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!("FluentManager localizer state lock poisoned while writing; recovering");
            poisoned.into_inner()
        },
    }
}

fn load_runtime_modules(
    discovered_modules: Vec<&'static dyn I18nModuleRegistration>,
) -> Vec<&'static dyn I18nModuleRegistration> {
    let mut modules = Vec::new();

    for module in discovered_modules {
        let data = module.data();
        tracing::info!("Discovered and loading i18n module: {}", data.name);
        match module.registration_kind() {
            ModuleRegistrationKind::RuntimeLocalizer => {
                modules.push(module);
            },
            ModuleRegistrationKind::MetadataOnly => {
                tracing::debug!(
                    "Skipping metadata-only i18n module '{}' for FluentManager runtime localization",
                    data.name
                );
            },
        }
    }

    modules
}

fn unexpected_missing_localizer(module: &ModuleData) -> crate::localization::LocalizationError {
    io::Error::other(format!(
        "runtime i18n module '{}' did not create a localizer during language selection",
        module.name
    ))
    .into()
}

fn format_module_discovery_errors(errors: Vec<ModuleDiscoveryError>) -> String {
    errors
        .into_iter()
        .map(|error| format!("- {error}"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl FluentManager {
    /// Creates a new `FluentManager` with strict discovered-module validation.
    ///
    /// This constructor fails fast when discovery finds invalid metadata or
    /// duplicate registrations. Use [`Self::best_effort_with_discovered_modules`]
    /// if you want discovery conflicts to be logged and skipped instead.
    pub fn new_with_discovered_modules() -> Self {
        Self::try_new_with_discovered_modules().unwrap_or_else(|errors| {
            panic!(
                "failed strict i18n module discovery:\n{}",
                format_module_discovery_errors(errors)
            )
        })
    }

    /// Creates a new `FluentManager` with lenient discovered-module validation.
    ///
    /// This keeps the legacy best-effort behavior: invalid metadata and
    /// unresolvable duplicates are logged and skipped.
    pub fn best_effort_with_discovered_modules() -> Self {
        let discovered_modules = filter_module_registry(
            inventory::iter::<&'static dyn I18nModuleRegistration>()
                .copied()
                .collect::<Vec<_>>(),
        );

        Self {
            modules: load_runtime_modules(discovered_modules),
            localizers: RwLock::default(),
        }
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

        Ok(Self {
            modules: load_runtime_modules(discovered_modules),
            localizers: RwLock::default(),
        })
    }

    /// Selects a language for all localizers.
    ///
    /// By default this is best-effort: modules that report
    /// `LanguageNotSupported` are skipped as long as at least one module
    /// accepts the requested locale.
    pub fn select_language(
        &self,
        lang: &LanguageIdentifier,
    ) -> crate::localization::LocalizationErrorResult<()> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::BestEffort)
    }

    /// Selects a language for all localizers and fails if any module rejects it.
    pub fn select_language_strict(
        &self,
        lang: &LanguageIdentifier,
    ) -> crate::localization::LocalizationErrorResult<()> {
        self.select_language_with_policy(lang, LanguageSelectionPolicy::Strict)
    }

    /// Selects a language for all localizers using the requested policy.
    pub fn select_language_with_policy(
        &self,
        lang: &LanguageIdentifier,
        policy: LanguageSelectionPolicy,
    ) -> crate::localization::LocalizationErrorResult<()> {
        let mut next_localizers = Vec::with_capacity(self.modules.len());
        let mut any_selected = false;
        let mut first_failure = None;
        let mut first_non_unsupported_failure = None;
        let mut unsupported_modules = Vec::new();

        for module in &self.modules {
            let data = module.data();
            let Some(localizer) = module.create_localizer() else {
                let error = unexpected_missing_localizer(data);
                tracing::error!(
                    "Module '{}' failed to create a runtime localizer for '{}': {}",
                    data.name,
                    lang,
                    error
                );
                if first_non_unsupported_failure.is_none() {
                    first_non_unsupported_failure = Some(error);
                }
                continue;
            };

            match localizer.select_language(lang) {
                Ok(()) => {
                    any_selected = true;
                    next_localizers.push((data, localizer));
                },
                Err(error) => {
                    tracing::debug!(
                        "Module '{}' failed to set language '{}': {}",
                        data.name,
                        lang,
                        error
                    );
                    if matches!(
                        &error,
                        crate::localization::LocalizationError::LanguageNotSupported(_)
                    ) {
                        unsupported_modules.push(data.name);
                        if first_failure.is_none() {
                            first_failure = Some(error);
                        }
                    } else if first_non_unsupported_failure.is_none() {
                        first_non_unsupported_failure = Some(error);
                    }
                },
            }
        }

        if let Some(error) = first_non_unsupported_failure {
            tracing::warn!(
                "Language selection for '{}' failed due to a runtime-localizer error; keeping the previous language active",
                lang,
            );
            return Err(error);
        }

        if any_selected && first_failure.is_some() && policy == LanguageSelectionPolicy::Strict {
            tracing::warn!(
                "Language selection for '{}' failed for at least one i18n module; keeping the previous language active",
                lang
            );
            return Err(first_failure.expect("selection failure should have been captured"));
        }

        if !any_selected {
            tracing::warn!(
                "No i18n modules support language '{}'; unsupported modules: {}",
                lang,
                unsupported_modules.join(", ")
            );
            return Err(crate::localization::LocalizationError::LanguageNotSupported(lang.clone()));
        }

        if !unsupported_modules.is_empty() {
            tracing::warn!(
                "Language '{}' is not supported by some i18n modules; continuing with: {}",
                lang,
                unsupported_modules.join(", ")
            );
        }

        *write_localizers(&self.localizers) = next_localizers;
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
        for (_, localizer) in read_localizers(&self.localizers).iter() {
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
        read_localizers(&self.localizers)
            .iter()
            .find(|(data, _)| data.domain == domain)
            .and_then(|(_, localizer)| localizer.localize(id, args))
    }
}
