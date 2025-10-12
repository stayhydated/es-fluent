use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FluentScParserError {
    /// An IO error.
    #[error("IO error accessing path '{0}': {1}")]
    Io(PathBuf, #[source] std::io::Error),

    /// An error that occurs when parsing a Rust file.
    #[error("Failed to parse Rust file '{0}': {1}")]
    Syn(PathBuf, #[source] syn::Error),

    /// An error that occurs when walking a directory.
    #[error("Error walking directory '{0}': {1}")]
    WalkDir(PathBuf, #[source] walkdir::Error),

    /// An error that occurs when parsing an attribute.
    #[error("Attribute parsing error in file '{0}': {1}")]
    AttributeParse(PathBuf, #[source] darling::Error),

    /// An error that occurs when a required attribute is missing.
    #[error("Missing required attribute data in file '{0}': {1}")]
    MissingAttribute(PathBuf, String),

    /// An internal logic error.
    #[error("Internal logic error: {0}")]
    Internal(String),
}
