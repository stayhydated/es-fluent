//! Shared namespace rules used by derive parsing and runtime registration.
use darling::FromMeta;
use std::{
    borrow::Cow,
    path::{Component, Path},
};

use crate::resource::{ResourceKey, ResourceKeyError};

/// Validation failures for resolved namespace paths.
#[derive(Clone, Copy, Debug, Eq, thiserror::Error, PartialEq)]
pub enum NamespacePathError {
    /// The namespace is empty after trimming.
    #[error("namespace must not be empty")]
    Empty,
    /// The namespace has leading or trailing whitespace.
    #[error("namespace must not have leading or trailing whitespace")]
    OuterWhitespace,
    /// The namespace uses a Windows-style separator.
    #[error("namespace must use '/' as path separator")]
    BackslashSeparator,
    /// The namespace contains an empty path segment.
    #[error("namespace path must not contain empty segments")]
    EmptySegment,
    /// The namespace contains `.` or `..`.
    #[error("namespace path must not contain '.' or '..' segments")]
    CurrentOrParentSegment,
    /// The namespace resolves as an absolute path.
    #[error("namespace must be a relative path")]
    AbsolutePath,
    /// The namespace includes the `.ftl` suffix.
    #[error("namespace must not include file extension")]
    FileExtension,
}

impl NamespacePathError {
    /// Returns the stable validation detail used by existing diagnostics.
    pub fn details(self) -> &'static str {
        match self {
            Self::Empty => "namespace must not be empty",
            Self::OuterWhitespace => "namespace must not have leading or trailing whitespace",
            Self::BackslashSeparator => "namespace must use '/' as path separator",
            Self::EmptySegment => "namespace path must not contain empty segments",
            Self::CurrentOrParentSegment => "namespace path must not contain '.' or '..' segments",
            Self::AbsolutePath => "namespace must be a relative path",
            Self::FileExtension => "namespace must not include file extension",
        }
    }
}

/// A namespace path that has been validated for locale-relative resource use.
#[derive(
    Clone, Debug, derive_more::AsRef, derive_more::Display, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
#[as_ref(str)]
pub struct ResolvedNamespace(Cow<'static, str>);

impl ResolvedNamespace {
    /// Creates a validated resolved namespace.
    pub fn new(namespace: impl Into<String>) -> Result<Self, NamespacePathError> {
        let namespace = namespace.into();
        validate_namespace_path_typed(&namespace)?;
        Ok(Self(Cow::Owned(namespace)))
    }

    /// Creates a namespace from a value already validated by macro expansion.
    pub(crate) const fn from_static_unchecked(namespace: &'static str) -> Self {
        Self(Cow::Borrowed(namespace))
    }

    /// Returns the namespace as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Returns the resource key for this namespace under the given domain.
    pub fn try_resource_key(&self, domain: &str) -> Result<ResourceKey, ResourceKeyError> {
        ResourceKey::try_new(format!("{domain}/{}", self.as_str()))
    }
}

impl PartialEq<&str> for ResolvedNamespace {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for ResolvedNamespace {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<ResolvedNamespace> for &str {
    fn eq(&self, other: &ResolvedNamespace) -> bool {
        *self == other.as_str()
    }
}

/// Namespace selection rules for FTL file output.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NamespaceRule {
    /// A literal namespace string.
    Literal(ResolvedNamespace),
    /// Use the source file name (stem only) as the namespace.
    File,
    /// Use the file path relative to the crate root as the namespace.
    FileRelative,
    /// Use the source file parent folder name as the namespace.
    Folder,
    /// Use the source file parent folder path relative to crate root as the namespace.
    FolderRelative,
}

impl NamespaceRule {
    /// Creates a literal namespace rule after validating the namespace path.
    pub fn literal(namespace: impl Into<String>) -> Result<Self, NamespacePathError> {
        ResolvedNamespace::new(namespace).map(Self::Literal)
    }

    /// Resolve the namespace string using the given file path.
    pub fn resolve(&self, file_path: &str, manifest_dir: Option<&Path>) -> String {
        match self {
            Self::Literal(value) => value.to_string(),
            Self::File => crate::namespace_resolver::file_stem_namespace(file_path),
            Self::FileRelative => {
                crate::namespace_resolver::file_relative_namespace(file_path, manifest_dir)
            },
            Self::Folder => crate::namespace_resolver::folder_namespace(file_path),
            Self::FolderRelative => {
                crate::namespace_resolver::folder_relative_namespace(file_path, manifest_dir)
            },
        }
    }

    /// Resolve and validate the namespace string using the given file path.
    pub fn try_resolve(
        &self,
        file_path: &str,
        manifest_dir: Option<&Path>,
    ) -> Result<ResolvedNamespace, NamespacePathError> {
        ResolvedNamespace::new(self.resolve(file_path, manifest_dir))
    }
}

/// Validate a resolved namespace before using it as a relative output path.
pub fn validate_namespace_path(namespace: &str) -> Result<(), &'static str> {
    validate_namespace_path_typed(namespace).map_err(NamespacePathError::details)
}

/// Validate a resolved namespace and return a typed validation failure.
pub fn validate_namespace_path_typed(namespace: &str) -> Result<(), NamespacePathError> {
    let trimmed = namespace.trim();
    if trimmed.is_empty() {
        return Err(NamespacePathError::Empty);
    }
    if namespace != trimmed {
        return Err(NamespacePathError::OuterWhitespace);
    }
    if trimmed.contains('\\') {
        return Err(NamespacePathError::BackslashSeparator);
    }
    if trimmed.split('/').any(|segment| segment.is_empty()) {
        return Err(NamespacePathError::EmptySegment);
    }
    if trimmed
        .split('/')
        .any(|segment| matches!(segment, "." | ".."))
    {
        return Err(NamespacePathError::CurrentOrParentSegment);
    }
    if Path::new(trimmed)
        .components()
        .any(|component| matches!(component, Component::RootDir | Component::Prefix(_)))
    {
        return Err(NamespacePathError::AbsolutePath);
    }
    if trimmed.ends_with(".ftl") {
        return Err(NamespacePathError::FileExtension);
    }

    Ok(())
}

impl FromMeta for NamespaceRule {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::NameValue(nv) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    ResolvedNamespace::new(s.value())
                        .map(Self::Literal)
                        .map_err(|error| darling::Error::custom(error.to_string()).with_span(s))
                } else if let syn::Expr::Path(path) = &nv.value {
                    parse_namespace_ident(path)
                } else {
                    Err(darling::Error::unexpected_type(
                        "expected string literal, 'file', 'file_relative', 'folder', or 'folder_relative'",
                    ))
                }
            },
            syn::Meta::List(_) => Err(darling::Error::unsupported_format(
                "expected namespace = \"value\", namespace = file, namespace = file_relative, namespace = folder, or namespace = folder_relative",
            )),
            _ => Err(darling::Error::unsupported_format(
                "expected namespace = \"value\", namespace = file, namespace = file_relative, namespace = folder, or namespace = folder_relative",
            )),
        }
    }
}

fn parse_namespace_ident(path: &syn::ExprPath) -> darling::Result<NamespaceRule> {
    let Some(ident) = path.path.get_ident() else {
        return Err(expected_namespace_value_error());
    };

    match ident.to_string().as_str() {
        "file" => Ok(NamespaceRule::File),
        "file_relative" => Ok(NamespaceRule::FileRelative),
        "folder" => Ok(NamespaceRule::Folder),
        "folder_relative" => Ok(NamespaceRule::FolderRelative),
        _ => Err(darling::Error::custom(
            "expected string literal, 'file', 'file_relative', 'folder', or 'folder_relative' identifier",
        )),
    }
}

fn expected_namespace_value_error() -> darling::Error {
    darling::Error::custom(
        "expected string literal, 'file', 'file_relative', 'folder', or 'folder_relative' identifier",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn literal_namespace_parses_and_resolves() {
        let meta: syn::Meta = parse_quote!(namespace = "my_namespace");
        let ns = NamespaceRule::from_meta(&meta).unwrap();
        assert!(matches!(ns, NamespaceRule::Literal(ref s) if s == "my_namespace"));
        assert_eq!(ns.resolve("/some/path/lib.rs", None), "my_namespace");
    }

    #[test]
    fn literal_namespace_constructor_accepts_static_str() {
        let ns = NamespaceRule::literal("ui").expect("valid namespace");
        assert_eq!(ns.resolve("/some/path/lib.rs", None), "ui");
    }

    #[test]
    fn file_and_folder_variants_parse() {
        let file_meta: syn::Meta = parse_quote!(namespace = file);
        assert!(matches!(
            NamespaceRule::from_meta(&file_meta).unwrap(),
            NamespaceRule::File
        ));

        let file_relative_meta: syn::Meta = parse_quote!(namespace = file_relative);
        assert!(matches!(
            NamespaceRule::from_meta(&file_relative_meta).unwrap(),
            NamespaceRule::FileRelative
        ));

        let folder_meta: syn::Meta = parse_quote!(namespace = folder);
        assert!(matches!(
            NamespaceRule::from_meta(&folder_meta).unwrap(),
            NamespaceRule::Folder
        ));

        let folder_relative_meta: syn::Meta = parse_quote!(namespace = folder_relative);
        assert!(matches!(
            NamespaceRule::from_meta(&folder_relative_meta).unwrap(),
            NamespaceRule::FolderRelative
        ));
    }

    #[test]
    fn namespace_rule_resolves_relative_variants() {
        assert_eq!(
            NamespaceRule::FileRelative.resolve("src/ui/button.rs", None),
            "ui/button"
        );
        assert_eq!(
            NamespaceRule::FolderRelative.resolve("src/ui/button.rs", None),
            "ui"
        );
    }

    #[test]
    fn resolved_namespace_builds_resource_keys() {
        let namespace = ResolvedNamespace::new("ui/button").unwrap();

        assert_eq!(namespace.as_str(), "ui/button");
        assert_eq!(
            namespace.try_resource_key("demo").unwrap().as_str(),
            "demo/ui/button"
        );
        assert_eq!(namespace.to_string(), "ui/button");
    }

    #[test]
    fn namespace_rule_try_resolve_validates_output() {
        let ns = NamespaceRule::FileRelative
            .try_resolve("src/ui/button.rs", None)
            .unwrap();
        assert_eq!(ns.as_str(), "ui/button");

        let err = NamespaceRule::literal("../escape").unwrap_err();
        assert_eq!(err, NamespacePathError::CurrentOrParentSegment);
        assert_eq!(
            err.details(),
            "namespace path must not contain '.' or '..' segments"
        );
    }

    #[test]
    fn relative_namespace_resolution_normalizes_parent_segments() {
        assert_eq!(
            NamespaceRule::FileRelative.resolve("src/ui/../button.rs", None),
            "button"
        );
        assert_eq!(
            NamespaceRule::FolderRelative.resolve("src/ui/../forms/button.rs", None),
            "forms"
        );
    }

    #[test]
    fn validate_namespace_path_rejects_unsafe_values() {
        assert!(validate_namespace_path("ui/button").is_ok());
        assert_eq!(
            validate_namespace_path_typed("../escape").unwrap_err(),
            NamespacePathError::CurrentOrParentSegment
        );
        assert_eq!(
            validate_namespace_path("").unwrap_err(),
            "namespace must not be empty"
        );
        assert_eq!(
            validate_namespace_path(" ui/button ").unwrap_err(),
            "namespace must not have leading or trailing whitespace"
        );
        assert_eq!(
            validate_namespace_path(r"ui\button").unwrap_err(),
            "namespace must use '/' as path separator"
        );
        assert_eq!(
            validate_namespace_path("ui//button").unwrap_err(),
            "namespace path must not contain empty segments"
        );
        assert_eq!(
            validate_namespace_path("../escape").unwrap_err(),
            "namespace path must not contain '.' or '..' segments"
        );
        assert_eq!(
            validate_namespace_path("/escape").unwrap_err(),
            "namespace path must not contain empty segments"
        );
        assert_eq!(
            validate_namespace_path("ui/button.ftl").unwrap_err(),
            "namespace must not include file extension"
        );
    }

    #[test]
    fn namespace_rule_rejects_unsupported_meta_shapes() {
        let unsupported_format: syn::Meta = parse_quote!(namespace);
        assert!(NamespaceRule::from_meta(&unsupported_format).is_err());

        let unknown_name_value_path: syn::Meta = parse_quote!(namespace = module);
        assert!(NamespaceRule::from_meta(&unknown_name_value_path).is_err());

        let unsupported_name_value_literal: syn::Meta = parse_quote!(namespace = 42);
        assert!(NamespaceRule::from_meta(&unsupported_name_value_literal).is_err());

        let unknown_target: syn::Meta = parse_quote!(namespace(module(relative)));
        assert!(NamespaceRule::from_meta(&unknown_target).is_err());

        let list_folder: syn::Meta = parse_quote!(namespace(folder));
        assert!(NamespaceRule::from_meta(&list_folder).is_err());

        let list_literal: syn::Meta = parse_quote!(namespace("ui/list"));
        assert!(NamespaceRule::from_meta(&list_literal).is_err());

        let unsupported_name_value_call: syn::Meta = parse_quote!(namespace = file(relative));
        assert!(NamespaceRule::from_meta(&unsupported_name_value_call).is_err());

        let multiple_arguments: syn::Meta = parse_quote!(namespace(file(relative, extra)));
        assert!(NamespaceRule::from_meta(&multiple_arguments).is_err());
    }
}
