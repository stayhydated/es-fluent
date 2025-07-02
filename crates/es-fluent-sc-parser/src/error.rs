use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FluentScParserError {
    #[error("IO error accessing path '{0}': {1}")]
    Io(PathBuf, #[source] std::io::Error),

    #[error("Failed to parse Rust file '{0}': {1}")]
    Syn(PathBuf, #[source] syn::Error),

    #[error("Error walking directory '{0}': {1}")]
    WalkDir(PathBuf, #[source] walkdir::Error),

    #[error("Attribute parsing error in file '{0}': {1}")]
    AttributeParse(PathBuf, #[source] darling::Error),

    #[error("Missing required attribute data in file '{0}': {1}")]
    MissingAttribute(PathBuf, String),

    #[error("Internal logic error: {0}")]
    Internal(String),
}
