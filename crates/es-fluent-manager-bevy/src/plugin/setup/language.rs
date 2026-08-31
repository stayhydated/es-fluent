use crate::I18nResource;
use bevy::log::{debug, info};
use es_fluent_manager_core::{FluentManager, LocalizationError, ModuleDiscoveryError};
use std::{collections::HashSet, sync::Arc};
use unic_langid::LanguageIdentifier;

pub(in crate::plugin) fn resolve_initial_language(
    requested_language: &LanguageIdentifier,
    discovered_languages: &HashSet<LanguageIdentifier>,
) -> LanguageIdentifier {
    let mut discovered_language_list = discovered_languages.iter().cloned().collect::<Vec<_>>();
    discovered_language_list.sort_by_key(|lang| lang.to_string());

    let resolved_language = es_fluent_manager_core::resolve_ready_locale(
        requested_language,
        &[],
        &discovered_language_list,
    )
    .unwrap_or_else(|| requested_language.clone());

    if resolved_language != *requested_language {
        info!(
            "Initial locale '{}' not found, falling back to '{}'",
            requested_language, resolved_language
        );
    }

    resolved_language
}

pub(in crate::plugin) fn initialize_i18n_resource(
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
) -> Result<I18nResource, String> {
    let discovered =
        FluentManager::try_discover_runtime_modules().map_err(format_module_discovery_errors)?;
    let fallback_manager = if discovered.is_empty() {
        None
    } else {
        Some(Arc::new(FluentManager::from_discovered_modules(
            &discovered,
        )))
    };

    initialize_i18n_resource_with_fallback_manager(
        requested_language,
        resolved_language,
        fallback_manager,
    )
}

pub(super) fn initialize_i18n_resource_with_fallback_manager(
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
    fallback_manager: Option<Arc<FluentManager>>,
) -> Result<I18nResource, String> {
    let i18n_resource = I18nResource::new_with_resolved_language(
        requested_language.clone(),
        resolved_language.clone(),
    );

    let Some(fallback_manager) = fallback_manager else {
        return Ok(i18n_resource);
    };

    if let Err(error) = select_fallback_manager_for_resolution(
        &fallback_manager,
        requested_language,
        resolved_language,
    ) {
        debug!(
            "Runtime fallback manager rejected initial locale '{}' resolved as '{}'; keeping it attached for future locale switches: {}",
            requested_language, resolved_language, error
        );
    }

    Ok(i18n_resource.with_fallback_manager(fallback_manager))
}

fn select_fallback_manager_for_resolution(
    fallback_manager: &FluentManager,
    requested_language: &LanguageIdentifier,
    resolved_language: &LanguageIdentifier,
) -> Result<(), LocalizationError> {
    match fallback_manager.select_language_for_supported_locale(requested_language) {
        Ok(()) => Ok(()),
        Err(requested_error) if resolved_language != requested_language => fallback_manager
            .select_language_for_supported_locale(resolved_language)
            .inspect_err(|_resolved_error| {
                debug!(
                    "Runtime fallback manager rejected requested locale '{}' before resolved locale '{}' failed: {}",
                    requested_language, resolved_language, requested_error
                );
            }),
        Err(error) => Err(error),
    }
}

fn format_module_discovery_errors(errors: Vec<ModuleDiscoveryError>) -> String {
    errors
        .into_iter()
        .map(|error| format!("- {error}"))
        .collect::<Vec<_>>()
        .join("\n")
}
