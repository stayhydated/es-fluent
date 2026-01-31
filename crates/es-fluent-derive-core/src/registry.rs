//! This module provides types for representing FTL variants and type information.

use crate::meta::TypeKind;
use std::convert::AsRef;
use std::path::{Component, Path};

/// Namespace selection rules for FTL file output.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NamespaceRule {
    /// A literal namespace string.
    Literal(&'static str),
    /// Use the source file name (stem only) as the namespace.
    File,
    /// Use the file path relative to the crate root as the namespace.
    FileRelative,
}

impl NamespaceRule {
    /// Resolve the namespace string at runtime using the given file path.
    pub fn resolve(self, file_path: &str, manifest_dir: &Path) -> String {
        match self {
            NamespaceRule::Literal(value) => value.to_string(),
            NamespaceRule::File => file_stem_namespace(file_path),
            NamespaceRule::FileRelative => file_relative_namespace(file_path, manifest_dir),
        }
    }
}

fn file_stem_namespace(file_path: &str) -> String {
    Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn file_relative_namespace(file_path: &str, manifest_dir: &Path) -> String {
    let path = Path::new(file_path);
    let mut relative = path;

    if path.is_absolute() {
        match path.strip_prefix(manifest_dir) {
            Ok(stripped) => relative = stripped,
            Err(_) => return file_stem_namespace(file_path),
        }
    }

    if let Ok(stripped) = relative.strip_prefix("src") {
        relative = stripped;
    }

    let mut without_ext = relative.to_path_buf();
    without_ext.set_extension("");
    path_to_namespace(&without_ext)
}

fn path_to_namespace(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => {
                parts.push(segment.to_string_lossy().into_owned());
            },
            Component::ParentDir => parts.push("..".to_string()),
            Component::CurDir => {},
            Component::RootDir | Component::Prefix(_) => {},
        }
    }

    if parts.is_empty() {
        "unknown".to_string()
    } else {
        parts.join("/")
    }
}

/// A variant representing a single FTL key entry.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlVariant {
    pub name: &'static str,
    pub ftl_key: &'static str,
    pub args: &'static [&'static str],
    /// The module path from `module_path!()`.
    pub module_path: &'static str,
    /// The line number from `line!()` macro.
    pub line: u32,
}

/// Type information for FTL registration, used by derive macros and the CLI.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlTypeInfo {
    pub type_kind: TypeKind,
    pub type_name: &'static str,
    pub variants: &'static [FtlVariant],
    /// The file path where this type is defined (from `file!()` macro).
    pub file_path: &'static str,
    /// The module path where this type is defined (from `module_path!()` macro).
    pub module_path: &'static str,
    /// Optional namespace for FTL file output. If Some, the type will be written to
    /// `{lang}/{crate}/{namespace}.ftl` instead of `{lang}/{crate}.ftl`.
    pub namespace: Option<NamespaceRule>,
}

impl AsRef<FtlTypeInfo> for FtlTypeInfo {
    fn as_ref(&self) -> &FtlTypeInfo {
        self
    }
}

impl FtlTypeInfo {
    /// Resolve the namespace for this type, if configured.
    pub fn resolved_namespace(&self, manifest_dir: &Path) -> Option<String> {
        self.namespace
            .map(|rule| rule.resolve(self.file_path, manifest_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::NamespaceRule;
    use std::path::PathBuf;

    fn test_manifest_dir() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"C:\repo\app")
        } else {
            PathBuf::from("/repo/app")
        }
    }

    #[test]
    fn file_namespace_uses_stem() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir.join("src").join("lib.rs");
        let namespace = NamespaceRule::File.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "lib");
    }

    #[test]
    fn file_relative_strips_manifest_and_src() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir.join("src").join("ui").join("button.rs");
        let namespace =
            NamespaceRule::FileRelative.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "ui/button");
    }

    #[test]
    fn file_relative_falls_back_to_stem_outside_manifest() {
        let manifest_dir = test_manifest_dir();
        let file_path = if cfg!(windows) {
            PathBuf::from(r"C:\other\src\lib.rs")
        } else {
            PathBuf::from("/other/src/lib.rs")
        };
        let namespace =
            NamespaceRule::FileRelative.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "lib");
    }
}
