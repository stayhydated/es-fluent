use es_fluent::GlobalLocalizationError;
use es_fluent_manager_core::ModuleDiscoveryError;

#[derive(Debug)]
pub enum DioxusInitError {
    ModuleDiscovery(Vec<ModuleDiscoveryError>),
    LanguageSelection(GlobalLocalizationError),
    GlobalLocalizer(GlobalLocalizationError),
}

impl std::fmt::Display for DioxusInitError {
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
            Self::GlobalLocalizer(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DioxusInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModuleDiscovery(_) => None,
            Self::LanguageSelection(error) | Self::GlobalLocalizer(error) => Some(error),
        }
    }
}
