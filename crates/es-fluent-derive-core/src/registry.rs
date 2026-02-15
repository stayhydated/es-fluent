//! This module provides types for representing FTL variants and type information.

use crate::meta::TypeKind;
use crate::namespace_resolver::{
    file_relative_namespace, file_stem_namespace, folder_namespace, folder_relative_namespace,
};
use std::convert::AsRef;
use std::path::Path;

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
            NamespaceRule::FileRelative => file_relative_namespace(file_path, Some(manifest_dir)),
            NamespaceRule::Folder => folder_namespace(file_path),
            NamespaceRule::FolderRelative => {
                folder_relative_namespace(file_path, Some(manifest_dir))
            },
        }
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
