use std::path::{Path, PathBuf};

use crate::CanonicalLanguageIdentifierError;
use crate::fluent::FluentIdentifierError;
use crate::namespace::NamespacePathError;

use super::{LocaleRelativeFtlPathError, ResourceKeyError};

/// Errors produced while building a module resource plan.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum ResourcePlanError {
    /// The module domain is not a valid Fluent domain.
    #[error("invalid Fluent domain '{domain}': {details}")]
    InvalidDomain {
        domain: String,
        details: FluentIdentifierError,
    },
    /// The module domain is not a valid resource key segment.
    #[error("invalid domain resource key '{key}': {details}")]
    InvalidResourceKey {
        /// Invalid resource key.
        key: String,
        /// Validation details.
        details: ResourceKeyError,
    },
    /// A generated locale-relative resource path is invalid.
    #[error("invalid locale-relative resource path '{path}': {details}")]
    InvalidResourcePath {
        /// Invalid locale-relative path.
        path: String,
        /// Validation details.
        details: LocaleRelativeFtlPathError,
    },
    /// A namespace entry is not a valid locale-relative namespace path.
    #[error("invalid namespace '{namespace}': {details}")]
    InvalidNamespace {
        namespace: String,
        details: NamespacePathError,
    },
}

/// Errors produced while discovering sparse resource plans from an assets tree.
#[derive(Debug, thiserror::Error)]
pub enum SparseAssetResourcePlanError {
    /// The locale assets root could not be read.
    #[error("Failed to read i18n directory at {path:?}: {source}")]
    ReadAssetsRoot {
        /// Assets root path.
        path: PathBuf,
        /// Filesystem error details.
        source: std::io::Error,
    },
    /// A directory entry under the locale assets root could not be read.
    #[error("Failed to read directory entry in {path:?}: {source}")]
    ReadAssetsRootEntry {
        /// Parent directory path.
        path: PathBuf,
        /// Filesystem error details.
        source: std::io::Error,
    },
    /// A namespace directory could not be read.
    #[error("Failed to read namespace directory {path:?}: {source}")]
    ReadNamespaceDirectory {
        /// Namespace directory path.
        path: PathBuf,
        /// Filesystem error details.
        source: std::io::Error,
    },
    /// A namespace directory entry could not be read.
    #[error("Failed to read directory entry in {path:?}: {source}")]
    ReadNamespaceDirectoryEntry {
        /// Parent directory path.
        path: PathBuf,
        /// Filesystem error details.
        source: std::io::Error,
    },
    /// A locale directory name is not UTF-8.
    #[error("Locale directory {path:?} contains a non-UTF-8 name")]
    NonUtf8LocaleDirectory {
        /// Locale directory path.
        path: PathBuf,
    },
    /// A locale directory is not a valid canonical BCP-47 language identifier.
    #[error("{}", format_locale_directory_error(raw_name, path, details))]
    InvalidLocaleDirectory {
        /// Raw directory name.
        raw_name: String,
        /// Locale directory path.
        path: PathBuf,
        /// Language parsing details.
        details: CanonicalLanguageIdentifierError,
    },
    /// A namespace path could not be made relative to the namespace root.
    #[error("Failed to derive namespace for asset {path:?} relative to {root:?}: {source}")]
    NamespaceRelativePath {
        /// FTL asset path.
        path: PathBuf,
        /// Namespace root path.
        root: PathBuf,
        /// Relative path error details.
        source: std::path::StripPrefixError,
    },
    /// A namespace path contains a non-UTF-8 component.
    #[error("Namespace path {path:?} contains non-UTF-8 components")]
    NonUtf8NamespacePath {
        /// Namespace path without the `.ftl` extension.
        path: PathBuf,
    },
    /// A discovered namespace is not a canonical namespace path.
    #[error("Discovered invalid namespace '{namespace}' in assets for crate '{domain}': {details}")]
    InvalidNamespace {
        /// Discovered namespace path.
        namespace: String,
        /// Module domain.
        domain: String,
        /// Namespace validation details.
        details: NamespacePathError,
    },
}

fn format_locale_directory_error(
    raw_name: &str,
    path: &Path,
    details: &CanonicalLanguageIdentifierError,
) -> String {
    match details {
        CanonicalLanguageIdentifierError::Invalid { source, .. } => format!(
            "Locale directory '{raw_name}' under \"{}\" is not a valid BCP-47 identifier: {source}",
            path.display()
        ),
        CanonicalLanguageIdentifierError::IcuInvalid { details, .. } => format!(
            "Locale directory '{raw_name}' under \"{}\" could not be parsed as an ICU locale: {details}",
            path.display()
        ),
        CanonicalLanguageIdentifierError::NonCanonical { canonical, .. } => format!(
            "Locale directory '{raw_name}' under \"{}\" must use canonical BCP-47 form '{canonical}'",
            path.display()
        ),
    }
}
