use super::localizer::LoadedDioxusI18nAssetModule;

use es_fluent_manager_core::{LocalizationError, ModuleDiscoveryError, ResourceLoadError};

use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum DioxusAssetLoadError {
    ModuleDiscovery(Arc<[ModuleDiscoveryError]>),
    LanguageSelection {
        error: Arc<LocalizationError>,
        resource_errors: Arc<[ResourceLoadError]>,
    },
}

impl DioxusAssetLoadError {
    pub(super) fn language_selection(
        error: LocalizationError,
        modules: &[LoadedDioxusI18nAssetModule],
    ) -> Self {
        let resource_errors = modules
            .iter()
            .flat_map(LoadedDioxusI18nAssetModule::resource_errors)
            .cloned()
            .collect::<Vec<_>>();

        Self::LanguageSelection {
            error: Arc::new(error),
            resource_errors: resource_errors.into(),
        }
    }

    pub fn resource_errors(&self) -> &[ResourceLoadError] {
        match self {
            Self::ModuleDiscovery(_) => &[],
            Self::LanguageSelection {
                resource_errors, ..
            } => resource_errors,
        }
    }
}

impl std::fmt::Display for DioxusAssetLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDiscovery(errors) => {
                f.write_str("failed strict i18n module discovery")?;
                for error in errors.iter() {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            },
            Self::LanguageSelection { error, .. } => {
                write!(f, "failed to select the requested language: {error}")
            },
        }
    }
}

impl std::error::Error for DioxusAssetLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection { error, .. } => Some(error.as_ref()),
        }
    }
}
