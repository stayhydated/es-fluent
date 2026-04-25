use crate::DioxusGlobalLocalizerError;
use es_fluent::GlobalLocalizationError;
use es_fluent_manager_core::ModuleDiscoveryError;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum DioxusInitError {
    ModuleDiscovery(Arc<[ModuleDiscoveryError]>),
    LanguageSelection(Arc<GlobalLocalizationError>),
    GlobalLocalizer(Arc<DioxusGlobalLocalizerError>),
}

impl DioxusInitError {
    pub(crate) fn module_discovery(errors: Vec<ModuleDiscoveryError>) -> Self {
        Self::ModuleDiscovery(errors.into())
    }

    pub(crate) fn language_selection(error: GlobalLocalizationError) -> Self {
        Self::LanguageSelection(Arc::new(error))
    }

    #[cfg(any(
        feature = "desktop",
        feature = "mobile",
        feature = "ssr",
        feature = "web"
    ))]
    pub(crate) fn global_localizer(error: DioxusGlobalLocalizerError) -> Self {
        Self::GlobalLocalizer(Arc::new(error))
    }
}

impl std::fmt::Display for DioxusInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModuleDiscovery(errors) => {
                f.write_str("failed strict i18n module discovery")?;
                for error in errors.iter() {
                    write!(f, "\n- {error}")?;
                }
                Ok(())
            },
            Self::LanguageSelection(error) => {
                write!(f, "failed to select the requested language: {error}")
            },
            Self::GlobalLocalizer(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DioxusInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection(error) => Some(error.as_ref()),
            Self::GlobalLocalizer(error) => Some(error.as_ref()),
        }
    }
}
