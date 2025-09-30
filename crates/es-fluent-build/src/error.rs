use thiserror::Error;

#[derive(Debug, Error)]
pub enum FluentBuildError {
    #[error(
        "No i18n.toml configuration file found at {0}. Please create one before using this tool."
    )]
    NoI18nConfig(std::path::PathBuf),

    #[error("Failed to parse i18n.toml configuration: {0}")]
    ConfigParseError(#[from] es_fluent_toml::I18nConfigError),

    #[error("Cannot create i18n output directory: {0}")]
    CreateDirError(#[from] std::io::Error),

    #[error(transparent)]
    FluentParserError(#[from] es_fluent_sc_parser::error::FluentScParserError),

    #[error(transparent)]
    FluentGenerateError(#[from] es_fluent_generate::error::FluentGenerateError),
}
