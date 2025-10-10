use thiserror::Error;

#[derive(Debug, Error)]
pub enum FluentGenerateError {
    /// An IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// An environment variable error.
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    /// An error that occurs when the package name is missing.
    #[error("Missing package name")]
    MissingPackageName,

    /// An error that occurs when parsing a Fluent file.
    #[error("Fluent parsing error: {0:?}")]
    ParseError(Vec<fluent_syntax::parser::ParserError>),

    /// An error that occurs when serializing a Fluent file.
    #[error("Fluent serialization error: {0}")]
    SerializeError(#[from] std::fmt::Error),
}
