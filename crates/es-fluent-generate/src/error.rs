use thiserror::Error;

#[derive(Debug, Error)]
pub enum FluentGenerateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),

    #[error("Missing package name")]
    MissingPackageName,

    #[error("Fluent parsing error: {0:?}")]
    ParseError(Vec<fluent_syntax::parser::ParserError>),

    #[error("Fluent serialization error: {0}")]
    SerializeError(#[from] std::fmt::Error),
}
