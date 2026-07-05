pub use es_fluent_generate::error::FluentGenerateError;

/// Error type for FTL generation.
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// Failed to read i18n.toml configuration.
    #[error("Configuration error: {0}")]
    Config(#[from] es_fluent_toml::I18nConfigError),

    /// Failed to detect crate name.
    #[error("Failed to detect crate name: {0}")]
    CrateName(String),

    /// Failed to generate FTL files.
    #[error("Generation error: {0}")]
    Generate(#[from] FluentGenerateError),

    /// Invalid namespace used (not in allowed list).
    #[error(
        "Invalid namespace '{namespace}' for type '{type_name}'. Allowed namespaces: {allowed:?}"
    )]
    InvalidNamespace {
        namespace: String,
        type_name: String,
        allowed: Vec<String>,
    },

    /// Invalid namespace path used by a registered type.
    #[error("Invalid namespace '{namespace}' for type '{type_name}': {details}")]
    InvalidNamespacePath {
        namespace: String,
        type_name: String,
        details: es_fluent_shared::namespace::NamespacePathError,
    },

    /// Failed to inspect locale directories.
    #[error("Locale discovery error: {0}")]
    RunnerIo(#[from] es_fluent_runner::RunnerIoError),
}
