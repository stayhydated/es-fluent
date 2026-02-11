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
    /// Use the source file parent folder name as the namespace.
    Folder,
    /// Use the source file parent folder path relative to the crate root as the namespace.
    FolderRelative,
}

impl NamespaceRule {
    /// Resolve the namespace string at runtime using the given file path.
    pub fn resolve(self, file_path: &str, manifest_dir: &Path) -> String {
        match self {
            NamespaceRule::Literal(value) => value.to_string(),
            NamespaceRule::File => file_stem_namespace(file_path),
            NamespaceRule::FileRelative => file_relative_namespace(file_path, manifest_dir),
            NamespaceRule::Folder => folder_namespace(file_path),
            NamespaceRule::FolderRelative => folder_relative_namespace(file_path, manifest_dir),
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

    let mut without_ext = relative.to_path_buf();
    without_ext.set_extension("");

    let namespace_path = strip_src_prefix_if_not_empty(&without_ext);
    path_to_namespace(&namespace_path)
}

fn folder_namespace(file_path: &str) -> String {
    Path::new(file_path)
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn folder_relative_namespace(file_path: &str, manifest_dir: &Path) -> String {
    let path = Path::new(file_path);
    let mut relative = path;

    if path.is_absolute() {
        match path.strip_prefix(manifest_dir) {
            Ok(stripped) => relative = stripped,
            Err(_) => return folder_namespace(file_path),
        }
    }

    let parent = relative.parent().unwrap_or(relative);
    let namespace_path = strip_src_prefix_if_not_empty(parent);
    path_to_namespace(&namespace_path)
}

fn strip_src_prefix_if_not_empty(path: &Path) -> std::path::PathBuf {
    if let Ok(stripped) = path.strip_prefix("src")
        && stripped.components().next().is_some()
    {
        return stripped.to_path_buf();
    }

    path.to_path_buf()
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

    #[test]
    fn folder_namespace_uses_parent_folder_name() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir
            .join("src")
            .join("ui")
            .join("forms")
            .join("button.rs");
        let namespace = NamespaceRule::Folder.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "forms");
    }

    #[test]
    fn folder_relative_strips_manifest_and_src() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir
            .join("src")
            .join("ui")
            .join("forms")
            .join("button.rs");
        let namespace =
            NamespaceRule::FolderRelative.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "ui/forms");
    }

    #[test]
    fn folder_relative_keeps_src_for_root_module() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir.join("src").join("lib.rs");
        let namespace =
            NamespaceRule::FolderRelative.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "src");
    }

    #[test]
    fn folder_relative_falls_back_to_parent_outside_manifest() {
        let manifest_dir = test_manifest_dir();
        let file_path = if cfg!(windows) {
            PathBuf::from(r"C:\other\src\lib.rs")
        } else {
            PathBuf::from("/other/src/lib.rs")
        };
        let namespace =
            NamespaceRule::FolderRelative.resolve(&file_path.to_string_lossy(), &manifest_dir);
        assert_eq!(namespace, "src");
    }
}
