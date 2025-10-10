use thiserror::Error;

/// An error that can occur in the CLI.
#[derive(Debug, Error)]
pub enum CliError {
    /// An error that occurs when parsing the `i18n.toml` configuration file.
    #[error("Failed to parse i18n.toml configuration: {0}")]
    ConfigParse(String),

    /// An error that occurs when creating the i18n output directory.
    #[error("Cannot create i18n output directory: {0}")]
    CreateDir(#[from] std::io::Error),

    /// An error that occurs when discovering the workspace metadata.
    #[error("Failed to discover workspace metadata: {0}")]
    WorkspaceDiscovery(String),

    /// An error that occurs when watching files.
    #[error("File watching error: {0}")]
    Watch(#[from] notify::Error),

    /// An error that occurs when parsing Fluent files.
    #[error("Fluent parser error: {0}")]
    FluentParser(String),

    /// An error that occurs when generating Fluent files.
    #[error("Fluent generation error: {0}")]
    FluentGenerate(String),

    /// An internal application error.
    #[error("An internal application error occurred: {0}")]
    Internal(String),
}

impl From<es_fluent_toml::I18nConfigError> for CliError {
    fn from(err: es_fluent_toml::I18nConfigError) -> Self {
        CliError::ConfigParse(err.to_string())
    }
}

impl From<cargo_metadata::Error> for CliError {
    fn from(err: cargo_metadata::Error) -> Self {
        CliError::WorkspaceDiscovery(err.to_string())
    }
}

impl From<es_fluent_sc_parser::error::FluentScParserError> for CliError {
    fn from(err: es_fluent_sc_parser::error::FluentScParserError) -> Self {
        CliError::FluentParser(err.to_string())
    }
}

impl From<es_fluent_generate::error::FluentGenerateError> for CliError {
    fn from(err: es_fluent_generate::error::FluentGenerateError) -> Self {
        CliError::FluentGenerate(err.to_string())
    }
}
