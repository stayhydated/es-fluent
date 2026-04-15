use super::I18nModuleRegistration;
use crate::asset_localization::{ModuleData, validate_module_registry};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleRegistrationKind {
    MetadataOnly,
    RuntimeLocalizer,
}

impl std::fmt::Display for ModuleRegistrationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MetadataOnly => f.write_str("metadata-only"),
            Self::RuntimeLocalizer => f.write_str("runtime-localizer"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleDiscoveryError {
    InvalidMetadata(crate::asset_localization::ModuleRegistryError),
    InconsistentModuleMetadata {
        name: String,
        domain: String,
    },
    DuplicateModuleRegistration {
        name: String,
        domain: String,
        kind: ModuleRegistrationKind,
        count: usize,
    },
}

impl std::fmt::Display for ModuleDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMetadata(error) => write!(f, "{error}"),
            Self::InconsistentModuleMetadata { name, domain } => write!(
                f,
                "module '{name}' (domain '{domain}') has mismatched metadata between registrations",
            ),
            Self::DuplicateModuleRegistration {
                name,
                domain,
                kind,
                count,
            } => write!(
                f,
                "module '{name}' (domain '{domain}') has {count} duplicate {kind} registrations",
            ),
        }
    }
}

impl std::error::Error for ModuleDiscoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidMetadata(error) => Some(error),
            Self::InconsistentModuleMetadata { .. } => None,
            Self::DuplicateModuleRegistration { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RegistrationCounts {
    metadata_only: usize,
    runtime_localizer: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct IdentityInspection {
    chosen_data: Option<&'static ModuleData>,
    metadata_only: Option<&'static ModuleData>,
    runtime_localizer: Option<&'static ModuleData>,
    counts: RegistrationCounts,
}

fn validate_single_module_data(data: &'static ModuleData, errors: &mut Vec<ModuleDiscoveryError>) {
    if let Err(validation_errors) = validate_module_registry([data]) {
        errors.extend(
            validation_errors
                .into_iter()
                .map(ModuleDiscoveryError::InvalidMetadata),
        );
    }
}

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
    let discovered_data = inspect_module_registry(&modules)
        .into_values()
        .filter_map(|inspection| inspection.chosen_data)
        .collect::<Vec<_>>();

    if let Err(errors) = validate_module_registry(discovered_data.iter().copied()) {
        for error in errors {
            tracing::error!("Invalid i18n module registry entry: {}", error);
        }
    }

    let mut filtered: Vec<&'static dyn I18nModuleRegistration> = Vec::with_capacity(modules.len());
    let mut filtered_kinds: Vec<ModuleRegistrationKind> = Vec::with_capacity(modules.len());
    let mut seen_module_names: HashMap<&'static str, usize> = HashMap::new();
    let mut seen_domains: HashMap<&'static str, usize> = HashMap::new();

    for module in modules {
        let data = module.data();
        let module_kind = module.registration_kind();
        if data.name.trim().is_empty() || data.domain.trim().is_empty() {
            tracing::warn!(
                "Skipping i18n module with invalid metadata: name='{}', domain='{}'",
                data.name,
                data.domain
            );
            continue;
        }
        if let Some(&existing_index) = seen_module_names.get(data.name) {
            let existing_data = filtered[existing_index].data();
            let existing_kind = filtered_kinds[existing_index];
            if existing_data.domain != data.domain {
                tracing::warn!(
                    "Skipping duplicate i18n module name '{}' (domain '{}')",
                    data.name,
                    data.domain
                );
                continue;
            }

            if existing_kind == ModuleRegistrationKind::MetadataOnly
                && module_kind == ModuleRegistrationKind::RuntimeLocalizer
            {
                if existing_data != data {
                    tracing::warn!(
                        "Replacing metadata-only i18n module '{}' with runtime-localizer registration despite mismatched metadata",
                        data.name
                    );
                }
                tracing::warn!(
                    "Replacing metadata-only i18n module '{}' with runtime-localizer registration",
                    data.name
                );
                filtered[existing_index] = module;
                filtered_kinds[existing_index] = module_kind;
            } else {
                if existing_data != data {
                    tracing::warn!(
                        "Skipping duplicate i18n module name '{}' (domain '{}') with mismatched metadata",
                        data.name,
                        data.domain
                    );
                    continue;
                }
                tracing::warn!(
                    "Skipping duplicate i18n module name '{}' (domain '{}')",
                    data.name,
                    data.domain
                );
            }
            continue;
        }

        if let Some(&existing_index) = seen_domains.get(data.domain) {
            let existing_data = filtered[existing_index].data();
            let existing_kind = filtered_kinds[existing_index];
            if existing_data.name == data.name {
                if existing_kind == ModuleRegistrationKind::MetadataOnly
                    && module_kind == ModuleRegistrationKind::RuntimeLocalizer
                {
                    if existing_data != data {
                        tracing::warn!(
                            "Replacing metadata-only i18n module '{}' with runtime-localizer registration despite mismatched metadata",
                            data.name
                        );
                    }
                    tracing::warn!(
                        "Replacing metadata-only i18n module '{}' with runtime-localizer registration",
                        data.name
                    );
                    filtered[existing_index] = module;
                    filtered_kinds[existing_index] = module_kind;
                } else {
                    if existing_data != data {
                        tracing::warn!(
                            "Skipping duplicate i18n module name '{}' (domain '{}') with mismatched metadata",
                            data.name,
                            data.domain
                        );
                        continue;
                    }
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
        filtered_kinds.push(module_kind);
    }

    filtered
}

/// Validates discovered registrations strictly and returns either a normalized
/// module list or the collected registry/discovery errors.
///
/// Strict validation still allows one metadata-only registration plus one
/// runtime-localizer registration for the same exact (`name`, `domain`)
/// identity, because that pairing is used intentionally by some integrations.
/// It rejects repeated registrations of the same kind for one identity and all
/// metadata validation failures reported by `validate_module_registry()`.
pub fn try_filter_module_registry(
    modules: impl IntoIterator<Item = &'static dyn I18nModuleRegistration>,
) -> Result<Vec<&'static dyn I18nModuleRegistration>, Vec<ModuleDiscoveryError>> {
    let modules = modules.into_iter().collect::<Vec<_>>();
    let inspections = inspect_module_registry(&modules);
    let discovered_data = inspections
        .values()
        .filter_map(|inspection| inspection.chosen_data)
        .collect::<Vec<_>>();

    let mut errors = Vec::new();
    if let Err(validation_errors) = validate_module_registry(discovered_data.iter().copied()) {
        errors.extend(
            validation_errors
                .into_iter()
                .map(ModuleDiscoveryError::InvalidMetadata),
        );
    }

    for ((name, domain), inspection) in inspections {
        if let (Some(metadata_only), Some(runtime_localizer)) =
            (inspection.metadata_only, inspection.runtime_localizer)
            && metadata_only != runtime_localizer
        {
            errors.push(ModuleDiscoveryError::InconsistentModuleMetadata {
                name: name.to_string(),
                domain: domain.to_string(),
            });

            if inspection.chosen_data != Some(metadata_only) {
                validate_single_module_data(metadata_only, &mut errors);
            }
            if inspection.chosen_data != Some(runtime_localizer) {
                validate_single_module_data(runtime_localizer, &mut errors);
            }
        }

        let counts = inspection.counts;
        if counts.metadata_only > 1 {
            errors.push(ModuleDiscoveryError::DuplicateModuleRegistration {
                name: name.to_string(),
                domain: domain.to_string(),
                kind: ModuleRegistrationKind::MetadataOnly,
                count: counts.metadata_only,
            });
        }
        if counts.runtime_localizer > 1 {
            errors.push(ModuleDiscoveryError::DuplicateModuleRegistration {
                name: name.to_string(),
                domain: domain.to_string(),
                kind: ModuleRegistrationKind::RuntimeLocalizer,
                count: counts.runtime_localizer,
            });
        }
    }

    if errors.is_empty() {
        Ok(filter_module_registry(modules))
    } else {
        Err(errors)
    }
}

fn inspect_module_registry(
    modules: &[&'static dyn I18nModuleRegistration],
) -> HashMap<(&'static str, &'static str), IdentityInspection> {
    let mut inspections: HashMap<(&'static str, &'static str), IdentityInspection> = HashMap::new();

    for module in modules {
        let data = module.data();
        let inspection = inspections.entry((data.name, data.domain)).or_default();
        inspection.chosen_data.get_or_insert(data);

        match module.registration_kind() {
            ModuleRegistrationKind::MetadataOnly => {
                inspection.counts.metadata_only += 1;
                inspection.metadata_only.get_or_insert(data);
            },
            ModuleRegistrationKind::RuntimeLocalizer => {
                inspection.counts.runtime_localizer += 1;
                inspection.runtime_localizer.get_or_insert(data);
            },
        }
    }

    inspections
}
