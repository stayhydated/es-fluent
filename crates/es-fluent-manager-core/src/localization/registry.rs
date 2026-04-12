use super::I18nModuleRegistration;
use crate::asset_localization::{ModuleData, validate_module_registry};
use std::collections::HashMap;

/// Normalizes discovered module registrations into a consistent, deduplicated list.
///
/// This applies shared validation and keeps only entries that satisfy:
/// - non-empty module name and domain
/// - unique module identity (`name` + `domain`)
/// - no conflicting duplicate names/domains
///
/// For exact duplicates (`name` + `domain`), runtime-localizer registrations are
/// preferred over metadata-only registrations.
pub fn filter_module_registry(
    modules: impl IntoIterator<Item = &'static dyn I18nModuleRegistration>,
) -> Vec<&'static dyn I18nModuleRegistration> {
    let modules = modules.into_iter().collect::<Vec<_>>();
    let mut discovered_data_by_identity: HashMap<
        (&'static str, &'static str),
        &'static ModuleData,
    > = HashMap::new();
    for module in &modules {
        let data = module.data();
        discovered_data_by_identity
            .entry((data.name, data.domain))
            .or_insert(data);
    }
    let discovered_data = discovered_data_by_identity
        .into_values()
        .collect::<Vec<_>>();

    if let Err(errors) = validate_module_registry(discovered_data.iter().copied()) {
        for error in errors {
            tracing::error!("Invalid i18n module registry entry: {}", error);
        }
    }

    let mut filtered: Vec<&'static dyn I18nModuleRegistration> = Vec::with_capacity(modules.len());
    let mut seen_module_names: HashMap<&'static str, usize> = HashMap::new();
    let mut seen_domains: HashMap<&'static str, usize> = HashMap::new();

    for module in modules {
        let data = module.data();
        if data.name.trim().is_empty() || data.domain.trim().is_empty() {
            tracing::warn!(
                "Skipping i18n module with invalid metadata: name='{}', domain='{}'",
                data.name,
                data.domain
            );
            continue;
        }
        if let Some(&existing_index) = seen_module_names.get(data.name) {
            let existing = filtered[existing_index];
            let existing_data = existing.data();
            if existing_data.domain != data.domain {
                tracing::warn!(
                    "Skipping duplicate i18n module name '{}' (domain '{}')",
                    data.name,
                    data.domain
                );
                continue;
            }

            if !existing.supports_runtime_localization() && module.supports_runtime_localization() {
                tracing::warn!(
                    "Replacing metadata-only i18n module '{}' with runtime-localizer registration",
                    data.name
                );
                filtered[existing_index] = module;
            } else {
                tracing::warn!(
                    "Skipping duplicate i18n module name '{}' (domain '{}')",
                    data.name,
                    data.domain
                );
            }
            continue;
        }

        if let Some(&existing_index) = seen_domains.get(data.domain) {
            let existing = filtered[existing_index];
            let existing_data = existing.data();
            if existing_data.name == data.name {
                if !existing.supports_runtime_localization()
                    && module.supports_runtime_localization()
                {
                    tracing::warn!(
                        "Replacing metadata-only i18n module '{}' with runtime-localizer registration",
                        data.name
                    );
                    filtered[existing_index] = module;
                } else {
                    tracing::warn!(
                        "Skipping duplicate i18n module name '{}' (domain '{}')",
                        data.name,
                        data.domain
                    );
                }
                continue;
            }

            tracing::warn!(
                "Skipping duplicate i18n domain '{}' from module '{}'",
                data.domain,
                data.name
            );
            continue;
        }

        let index = filtered.len();
        seen_module_names.insert(data.name, index);
        seen_domains.insert(data.domain, index);
        filtered.push(module);
    }

    filtered
}
