//! This module provides types for representing FTL variants and type information.

use crate::meta::TypeKind;
pub use crate::namespace::NamespaceRule;
use std::convert::AsRef;
use std::path::Path;

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
            .as_ref()
            .map(|rule| rule.resolve(self.file_path, Some(manifest_dir)))
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
        let namespace = NamespaceRule::File.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
        assert_eq!(namespace, "lib");
    }

    #[test]
    fn file_relative_strips_manifest_and_src() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir.join("src").join("ui").join("button.rs");
        let namespace = NamespaceRule::FileRelative.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
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
        let namespace = NamespaceRule::FileRelative.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
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
        let namespace = NamespaceRule::Folder.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
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
        let namespace = NamespaceRule::FolderRelative.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
        assert_eq!(namespace, "ui/forms");
    }

    #[test]
    fn folder_relative_keeps_src_for_root_module() {
        let manifest_dir = test_manifest_dir();
        let file_path = manifest_dir.join("src").join("lib.rs");
        let namespace = NamespaceRule::FolderRelative.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
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
        let namespace = NamespaceRule::FolderRelative.resolve(
            file_path.to_str().expect("utf-8 test path"),
            Some(&manifest_dir),
        );
        assert_eq!(namespace, "src");
    }
}
