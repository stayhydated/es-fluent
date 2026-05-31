//! This module provides types for representing FTL variants and type information.

use crate::fluent::{FluentArgumentName, FluentEntryId, FluentIdentifierError, FluentMessageId};
use crate::meta::TypeKind;
pub use crate::namespace::{NamespacePathError, NamespaceRule, ResolvedNamespace};
use crate::source::{SourceFile, SourceLine, SourceLocation};
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

impl FtlVariant {
    /// Returns the validated Fluent entry id for this variant.
    ///
    /// Unlike [`Self::message_id`], this accepts both message IDs and term IDs
    /// such as `-shared`.
    pub fn entry_id(&self) -> Result<FluentEntryId, FluentIdentifierError> {
        FluentEntryId::try_new(self.ftl_key)
    }

    /// Returns the validated Fluent message id for this variant.
    pub fn message_id(&self) -> Result<FluentMessageId, FluentIdentifierError> {
        FluentMessageId::try_new(self.ftl_key)
    }

    /// Returns the validated Fluent argument names for this variant.
    pub fn argument_names(&self) -> Result<Vec<FluentArgumentName>, FluentIdentifierError> {
        self.args
            .iter()
            .map(|arg| FluentArgumentName::try_new(*arg))
            .collect()
    }

    /// Returns typed source line metadata for this variant.
    pub fn source_line(&self) -> SourceLine {
        SourceLine::new(self.line)
    }
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
    /// Returns typed source file metadata when this type has a recorded file path.
    pub fn source_file(&self) -> Option<SourceFile> {
        SourceFile::new(self.file_path)
    }

    /// Returns typed source location metadata for the given variant.
    pub fn source_location_for(&self, variant: &FtlVariant) -> Option<SourceLocation> {
        SourceLocation::new(self.file_path, variant.line)
    }

    /// Resolve the namespace for this type, if configured.
    pub fn resolved_namespace(&self, manifest_dir: &Path) -> Option<String> {
        self.namespace
            .as_ref()
            .map(|rule| rule.resolve(self.file_path, Some(manifest_dir)))
    }

    /// Resolve and validate the namespace for this type, if configured.
    pub fn try_resolved_namespace(
        &self,
        manifest_dir: &Path,
    ) -> Result<Option<ResolvedNamespace>, NamespacePathError> {
        self.namespace
            .as_ref()
            .map(|rule| rule.try_resolve(self.file_path, Some(manifest_dir)))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::{FtlTypeInfo, NamespacePathError, NamespaceRule};
    use crate::meta::TypeKind;
    use crate::registry::FtlVariant;
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

    #[test]
    fn ftl_type_info_try_resolved_namespace_returns_validated_namespace() {
        let manifest_dir = test_manifest_dir();
        let info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "ButtonCopy",
            variants: &[],
            file_path: "src/ui/button.rs",
            module_path: "demo",
            namespace: Some(NamespaceRule::FileRelative),
        };

        let namespace = info
            .try_resolved_namespace(&manifest_dir)
            .expect("namespace should resolve")
            .expect("namespace configured");

        assert_eq!(namespace.as_str(), "ui/button");
        assert_eq!(namespace.resource_key("demo").as_str(), "demo/ui/button");
    }

    #[test]
    fn ftl_type_info_try_resolved_namespace_rejects_invalid_literal() {
        let manifest_dir = test_manifest_dir();
        let info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "EscapingCopy",
            variants: &[],
            file_path: "src/lib.rs",
            module_path: "demo",
            namespace: Some(NamespaceRule::Literal("../escape".into())),
        };

        let err = info
            .try_resolved_namespace(&manifest_dir)
            .expect_err("invalid literal should fail");

        assert_eq!(err, NamespacePathError::CurrentOrParentSegment);
    }

    #[test]
    fn ftl_type_info_exposes_typed_source_metadata() {
        static VARIANTS: &[FtlVariant] = &[FtlVariant {
            name: "Ready",
            ftl_key: "status-Ready",
            args: &[],
            module_path: "demo",
            line: 42,
        }];
        let info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "Status",
            variants: VARIANTS,
            file_path: "src/status.rs",
            module_path: "demo",
            namespace: None,
        };

        assert_eq!(info.source_file().unwrap().as_str(), "src/status.rs");
        assert_eq!(VARIANTS[0].entry_id().unwrap().as_str(), "status-Ready");
        assert_eq!(VARIANTS[0].message_id().unwrap().as_str(), "status-Ready");
        assert_eq!(VARIANTS[0].argument_names().unwrap(), Vec::new());
        assert_eq!(VARIANTS[0].source_line().get(), 42);

        let location = info.source_location_for(&VARIANTS[0]).unwrap();
        assert_eq!(location.file().as_str(), "src/status.rs");
        assert_eq!(location.line().get(), 42);
    }

    #[test]
    fn empty_type_file_path_has_no_typed_source_location() {
        static VARIANTS: &[FtlVariant] = &[FtlVariant {
            name: "Ready",
            ftl_key: "status-Ready",
            args: &[],
            module_path: "demo",
            line: 42,
        }];
        let info = FtlTypeInfo {
            type_kind: TypeKind::Enum,
            type_name: "Status",
            variants: VARIANTS,
            file_path: "",
            module_path: "demo",
            namespace: None,
        };

        assert!(info.source_file().is_none());
        assert!(info.source_location_for(&VARIANTS[0]).is_none());
    }

    #[test]
    fn ftl_variant_reports_invalid_message_ids_and_arguments() {
        let invalid_key = FtlVariant {
            name: "Broken",
            ftl_key: "_invalid",
            args: &[],
            module_path: "demo",
            line: 1,
        };
        assert_eq!(
            invalid_key.message_id().unwrap_err().to_string(),
            "Fluent message id must start with an ASCII letter"
        );
        assert_eq!(
            invalid_key.entry_id().unwrap_err().to_string(),
            "Fluent entry id must start with an ASCII letter"
        );

        let invalid_arg = FtlVariant {
            name: "Broken",
            ftl_key: "valid",
            args: &["not valid"],
            module_path: "demo",
            line: 1,
        };
        assert_eq!(
            invalid_arg.argument_names().unwrap_err().to_string(),
            "Fluent argument name contains invalid character ' '; use ASCII letters, digits, '_' or '-'"
        );
    }
}
