use super::{
    I18nModuleRegistration, LanguageSelectionPolicy, Localizer, ModuleDiscoveryError,
    ModuleRegistrationKind, try_filter_module_registry,
};
use crate::asset_localization::ModuleData;
use fluent_bundle::FluentValue;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io;
use unic_langid::LanguageIdentifier;

type ManagedLocalizer = (&'static ModuleData, Box<dyn Localizer>);
const MAX_DIAGNOSTIC_LANGUAGES: usize = 6;

/// A manager for Fluent translations.
#[derive(Default)]
pub struct FluentManager {
    pub(super) modules: Vec<&'static dyn I18nModuleRegistration>,
    pub(super) localizers: RwLock<Vec<ManagedLocalizer>>,
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

pub(crate) fn format_module_names(modules: &[&'static ModuleData]) -> String {
    if modules.is_empty() {
        return "<none>".to_string();
    }

    modules
        .iter()
        .map(|data| data.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn format_supported_languages(languages: &[LanguageIdentifier]) -> String {
    if languages.is_empty() {
        return "none declared".to_string();
    }

    let mut formatted = languages
        .iter()
        .take(MAX_DIAGNOSTIC_LANGUAGES)
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if languages.len() > MAX_DIAGNOSTIC_LANGUAGES {
        formatted.push(format!(
            "+{} more",
            languages.len() - MAX_DIAGNOSTIC_LANGUAGES
        ));
    }

    formatted.join(", ")
}

pub(crate) fn format_module_support(data: &ModuleData) -> String {
    if data.domain == data.name {
        return format!(
            "{} (supports: {})",
            data.name,
            format_supported_languages(data.supported_languages)
        );
    }

    format!(
        "{} (domain: {}, supports: {})",
        data.name,
        data.domain,
        format_supported_languages(data.supported_languages)
    )
}

pub(crate) fn format_module_support_list(modules: &[&'static ModuleData]) -> String {
    if modules.is_empty() {
        return "<none>".to_string();
    }

    modules
        .iter()
        .map(|data| format_module_support(data))
        .collect::<Vec<_>>()
        .join(", ")
}

impl FluentManager {
    /// Creates a new `FluentManager` with strict discovered-module validation.
    pub fn new_with_discovered_modules() -> Self {
        Self::try_new_with_discovered_modules().unwrap_or_else(|errors| {
            panic!(
                "failed strict i18n module discovery:\n{}",
                format_module_discovery_errors(errors)
            )
        })
    }

    /// Creates a new `FluentManager` with strict registry validation.
    ///
    /// This returns an error instead of panicking when discovery finds invalid
    /// module metadata or unresolvable duplicate registrations.
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
        let mut selected_modules = Vec::with_capacity(self.modules.len());
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
                    first_non_unsupported_failure = Some((data, error));
                }
                continue;
            };

            match localizer.select_language(lang) {
                Ok(()) => {
                    any_selected = true;
                    selected_modules.push(data);
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
                        unsupported_modules.push(data);
                        if first_failure.is_none() {
                            first_failure = Some(error);
                        }
                    } else if first_non_unsupported_failure.is_none() {
                        first_non_unsupported_failure = Some((data, error));
                    }
                },
            }
        }

        if let Some((module, error)) = first_non_unsupported_failure {
            tracing::warn!(
                "Language selection for '{}' failed because module '{}' returned a runtime-localizer error: {}; keeping the previous language active",
                lang,
                module.name,
                error,
            );
            return Err(error);
        }

        if any_selected
            && policy == LanguageSelectionPolicy::Strict
            && let Some(error) = first_failure
        {
            tracing::warn!(
                "Language selection for '{}' failed in strict mode; modules that accepted it: {}; modules that rejected it: {}; keeping the previous language active",
                lang,
                format_module_names(&selected_modules),
                format_module_support_list(&unsupported_modules),
            );
            return Err(error);
        }

        if !any_selected {
            tracing::warn!(
                "No i18n modules support language '{}'; modules checked: {}",
                lang,
                format_module_support_list(&unsupported_modules)
            );
            return Err(crate::localization::LocalizationError::LanguageNotSupported(lang.clone()));
        }

        if !unsupported_modules.is_empty() {
            tracing::warn!(
                "Language '{}' is only partially supported; active modules: {}; skipped unsupported modules: {}",
                lang,
                format_module_names(&selected_modules),
                format_module_support_list(&unsupported_modules),
            );
        }

        *self.localizers.write() = next_localizers;
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
        for (_, localizer) in self.localizers.read().iter() {
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
            .read()
            .iter()
            .find(|(data, _)| data.domain == domain)
            .and_then(|(_, localizer)| localizer.localize(id, args))
    }
}
