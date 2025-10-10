use thiserror::Error;

/// An error that can occur during the build process.
#[derive(Debug, Error)]
pub enum FluentBuildError {
    /// An error that occurs when the `i18n.toml` configuration file is not
    /// found.
    #[error(
        "No i18n.toml configuration file found at {0}. Please create one before using this tool."
    )]
    NoI18nConfig(std::path::PathBuf),

    /// An error that occurs when the `i18n.toml` configuration file cannot be
    /// parsed.
    #[error("Failed to parse i18n.toml configuration: {0}")]
    ConfigParseError(#[from] es_fluent_toml::I18nConfigError),

    /// An error that occurs when the i18n output directory cannot be created.
    #[error("Cannot create i18n output directory: {0}")]
    CreateDirError(#[from] std::io::Error),

    /// An error that occurs when parsing the Rust source code fails.
    #[error(transparent)]
    FluentParserError(#[from] es_fluent_sc_parser::error::FluentScParserError),

    /// An error that occurs when generating the Fluent translation file fails.
    #[error(transparent)]
    FluentGenerateError(#[from] es_fluent_generate::error::FluentGenerateError),
}
