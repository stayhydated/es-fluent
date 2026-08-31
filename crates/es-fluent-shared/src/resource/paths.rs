use crate::namespace::NamespacePathError;

/// Errors produced while validating locale-relative Fluent resource paths.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum LocaleRelativeFtlPathError {
    /// The path is empty.
    #[error("path must not be empty")]
    Empty,
    /// The path is absolute.
    #[error("path must be relative")]
    Absolute,
    /// The path contains a Windows path separator.
    #[error("path must use '/' separators")]
    Backslash,
    /// The path does not end with `.ftl`.
    #[error("path must end with .ftl")]
    MissingFtlSuffix,
    /// The path stem is not a valid locale-relative namespace-style path.
    #[error("{0}")]
    InvalidStem(#[from] NamespacePathError),
}

/// Locale-relative path to a Fluent resource file.
///
/// Paths use the canonical shape `{domain}.ftl` or
/// `{domain}/{namespace}.ftl` and are relative to a locale root.
#[derive(
    Clone,
    Debug,
    derive_more::AsRef,
    derive_more::Deref,
    derive_more::Display,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
#[as_ref(str)]
#[deref(forward)]
pub struct LocaleRelativeFtlPath(String);

impl LocaleRelativeFtlPath {
    /// Validates and creates a locale-relative Fluent resource path.
    pub fn try_new(path: impl Into<String>) -> Result<Self, LocaleRelativeFtlPathError> {
        let path = path.into();
        validate_locale_relative_ftl_path(&path)?;
        Ok(Self(path))
    }

    /// Validates and creates a locale-relative Fluent resource path from a static literal.
    #[allow(
        clippy::panic,
        reason = "static metadata uses literal paths; use try_new for dynamic input"
    )]
    pub fn from_static_path(path: &'static str) -> Self {
        Self::try_new(path)
            .unwrap_or_else(|error| panic!("invalid locale-relative FTL path '{path}': {error}"))
    }

    /// Returns the path as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl PartialEq<&str> for LocaleRelativeFtlPath {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<LocaleRelativeFtlPath> for &str {
    fn eq(&self, other: &LocaleRelativeFtlPath) -> bool {
        *self == other.as_str()
    }
}

impl serde::Serialize for LocaleRelativeFtlPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for LocaleRelativeFtlPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::try_new(value).map_err(serde::de::Error::custom)
    }
}

fn validate_locale_relative_ftl_path(path: &str) -> Result<(), LocaleRelativeFtlPathError> {
    if path.is_empty() {
        return Err(LocaleRelativeFtlPathError::Empty);
    }
    if path.starts_with('/') {
        return Err(LocaleRelativeFtlPathError::Absolute);
    }
    if path.contains('\\') {
        return Err(LocaleRelativeFtlPathError::Backslash);
    }

    let Some(stem) = path.strip_suffix(".ftl") else {
        return Err(LocaleRelativeFtlPathError::MissingFtlSuffix);
    };

    crate::namespace::validate_namespace_path_typed(stem)?;
    Ok(())
}
