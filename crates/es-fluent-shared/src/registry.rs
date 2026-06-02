//! This module provides types for representing FTL variants and type information.

use crate::fluent::{
    FluentArgumentName, FluentDomain, FluentEntryId, FluentIdentifierError, FluentMessageId,
};
use crate::meta::TypeKind;
pub use crate::namespace::{NamespacePathError, NamespaceRule, ResolvedNamespace};
use crate::source::{SourceFile, SourceLine, SourceLocation};
use std::convert::AsRef;
use std::path::Path;

/// Static Fluent domain emitted by derive macros.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StaticFluentDomain(&'static str);

impl StaticFluentDomain {
    /// Creates a static domain from a caller-validated value.
    ///
    /// Derive macros emit this only after validating the domain during macro
    /// expansion. Manual callers should prefer [`Self::try_new`].
    pub(crate) const fn new_unchecked(value: &'static str) -> Self {
        Self(value)
    }

    /// Validates and creates a static domain.
    pub fn try_new(value: &'static str) -> Result<Self, FluentIdentifierError> {
        FluentDomain::try_new(value)?;
        Ok(Self(value))
    }

    /// Validates and creates the default domain derived from `CARGO_PKG_NAME`.
    ///
    /// Generated code uses this for implicit current-package domains because
    /// the package name is only known in the consuming crate.
    #[allow(
        clippy::panic,
        clippy::unwrap_used,
        reason = "invalid package names should fail immediately when generated localization code is used"
    )]
    pub fn from_package_name(value: &'static str) -> Self {
        Self::try_new(value).unwrap_or_else(|error| {
            panic!("CARGO_PKG_NAME '{value}' is not a valid Fluent domain: {error}")
        })
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl AsRef<str> for StaticFluentDomain {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl std::fmt::Display for StaticFluentDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl PartialEq<&str> for StaticFluentDomain {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Static Fluent entry identifier emitted by derive macros.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StaticFluentEntryId(&'static str);

impl StaticFluentEntryId {
    /// Creates a static entry id from a caller-validated value.
    ///
    /// Derive macros emit this only after validating the id during macro
    /// expansion. Manual callers should prefer [`Self::try_new`].
    pub(crate) const fn new_unchecked(value: &'static str) -> Self {
        Self(value)
    }

    /// Validates and creates a static entry id.
    pub fn try_new(value: &'static str) -> Result<Self, FluentIdentifierError> {
        FluentEntryId::try_new(value)?;
        Ok(Self(value))
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }

    pub fn message_id(self) -> Result<FluentMessageId, FluentIdentifierError> {
        FluentMessageId::try_new(self.0)
    }

    pub fn entry_id(self) -> FluentEntryId {
        FluentEntryId::from_valid_static(self.0)
    }
}

impl AsRef<str> for StaticFluentEntryId {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl std::fmt::Display for StaticFluentEntryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl PartialEq<&str> for StaticFluentEntryId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Static Fluent argument name emitted by derive macros.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StaticFluentArgumentName(&'static str);

impl StaticFluentArgumentName {
    /// Creates a static argument name from a caller-validated value.
    ///
    /// Derive macros emit this only after validating the name during macro
    /// expansion. Manual callers should prefer [`Self::try_new`].
    pub(crate) const fn new_unchecked(value: &'static str) -> Self {
        Self(value)
    }

    /// Validates and creates a static argument name.
    pub fn try_new(value: &'static str) -> Result<Self, FluentIdentifierError> {
        FluentArgumentName::try_new(value)?;
        Ok(Self(value))
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }

    pub fn argument_name(self) -> FluentArgumentName {
        FluentArgumentName::from_valid_static(self.0)
    }
}

impl AsRef<str> for StaticFluentArgumentName {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl std::fmt::Display for StaticFluentArgumentName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl PartialEq<&str> for StaticFluentArgumentName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// A variant representing a single FTL key entry.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlVariant {
    name: &'static str,
    ftl_key: StaticFluentEntryId,
    args: &'static [StaticFluentArgumentName],
    /// The module path from `module_path!()`.
    module_path: &'static str,
    /// The line number from `line!()` macro.
    line: u32,
}

impl FtlVariant {
    /// Creates static variant metadata from validated static parts.
    pub const fn new(
        name: &'static str,
        ftl_key: StaticFluentEntryId,
        args: &'static [StaticFluentArgumentName],
        module_path: &'static str,
        line: u32,
    ) -> Self {
        Self {
            name,
            ftl_key,
            args,
            module_path,
            line,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn args(&self) -> &'static [StaticFluentArgumentName] {
        self.args
    }

    pub fn module_path(&self) -> &'static str {
        self.module_path
    }

    pub fn entry_id(&self) -> FluentEntryId {
        self.ftl_key.entry_id()
    }

    /// Returns the validated Fluent message id for this variant.
    pub fn message_id(&self) -> Result<FluentMessageId, FluentIdentifierError> {
        self.ftl_key.message_id()
    }

    /// Returns the validated Fluent argument names for this variant.
    pub fn argument_names(&self) -> Vec<FluentArgumentName> {
        self.args.iter().map(|arg| arg.argument_name()).collect()
    }

    /// Returns typed source line metadata for this variant.
    pub fn source_line(&self) -> SourceLine {
        SourceLine::new(self.line)
    }
}

/// Type information for FTL registration, used by derive macros and the CLI.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlTypeInfo {
    type_kind: TypeKind,
    type_name: &'static str,
    variants: &'static [FtlVariant],
    /// The file path where this type is defined (from `file!()` macro).
    file_path: &'static str,
    /// The module path where this type is defined (from `module_path!()` macro).
    module_path: &'static str,
    /// Optional namespace for FTL file output. If Some, the type will be written to
    /// `{lang}/{crate}/{namespace}.ftl` instead of `{lang}/{crate}.ftl`.
    namespace: Option<NamespaceRule>,
}

impl AsRef<FtlTypeInfo> for FtlTypeInfo {
    fn as_ref(&self) -> &FtlTypeInfo {
        self
    }
}

impl FtlTypeInfo {
    /// Creates static type metadata from validated static parts.
    pub const fn new(
        type_kind: TypeKind,
        type_name: &'static str,
        variants: &'static [FtlVariant],
        file_path: &'static str,
        module_path: &'static str,
        namespace: Option<NamespaceRule>,
    ) -> Self {
        Self {
            type_kind,
            type_name,
            variants,
            file_path,
            module_path,
            namespace,
        }
    }

    pub fn type_kind(&self) -> &TypeKind {
        &self.type_kind
    }

    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    pub fn variants(&self) -> &'static [FtlVariant] {
        self.variants
    }

    pub fn file_path(&self) -> &'static str {
        self.file_path
    }

    pub fn module_path(&self) -> &'static str {
        self.module_path
    }

    pub fn namespace(&self) -> Option<&NamespaceRule> {
        self.namespace.as_ref()
    }

    /// Returns typed source file metadata when this type has a recorded file path.
    pub fn source_file(&self) -> Option<SourceFile> {
        SourceFile::new(self.file_path)
    }

    /// Returns typed source location metadata for the given variant.
    pub fn source_location_for(&self, variant: &FtlVariant) -> Option<SourceLocation> {
        SourceLocation::new(self.file_path, variant.line)
    }

    /// Returns a stable human-readable source description for diagnostics.
    pub fn source_description_for(&self, variant: &FtlVariant) -> String {
        let item = format!("type '{}' variant '{}'", self.type_name, variant.name);
        match self.source_location_for(variant) {
            Some(location) => format!(
                "{item} at {}:{}",
                location.file().as_str(),
                location.line().get()
            ),
            None => format!("{item} at line {}", variant.source_line().get()),
        }
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

/// Constructors used by generated macro output.
///
/// These functions keep generated metadata on a narrow construction surface
/// while the public structs expose read-only accessors.
#[doc(hidden)]
pub mod __macro {
    use super::{
        FtlTypeInfo, FtlVariant, NamespaceRule, ResolvedNamespace, StaticFluentArgumentName,
        StaticFluentDomain, StaticFluentEntryId,
    };
    use crate::meta::TypeKind;

    pub const fn static_domain(value: &'static str) -> StaticFluentDomain {
        StaticFluentDomain::new_unchecked(value)
    }

    pub const fn static_entry_id(value: &'static str) -> StaticFluentEntryId {
        StaticFluentEntryId::new_unchecked(value)
    }

    pub const fn static_argument_name(value: &'static str) -> StaticFluentArgumentName {
        StaticFluentArgumentName::new_unchecked(value)
    }

    pub const fn namespace_literal(value: &'static str) -> NamespaceRule {
        NamespaceRule::Literal(ResolvedNamespace::from_static_unchecked(value))
    }

    pub const fn ftl_variant(
        name: &'static str,
        ftl_key: StaticFluentEntryId,
        args: &'static [StaticFluentArgumentName],
        module_path: &'static str,
        line: u32,
    ) -> FtlVariant {
        FtlVariant::new(name, ftl_key, args, module_path, line)
    }

    pub const fn ftl_type_info(
        type_kind: TypeKind,
        type_name: &'static str,
        variants: &'static [FtlVariant],
        file_path: &'static str,
        module_path: &'static str,
        namespace: Option<NamespaceRule>,
    ) -> FtlTypeInfo {
        FtlTypeInfo::new(
            type_kind,
            type_name,
            variants,
            file_path,
            module_path,
            namespace,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FtlTypeInfo, NamespacePathError, NamespaceRule, StaticFluentArgumentName,
        StaticFluentDomain, StaticFluentEntryId,
    };
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
        let info = FtlTypeInfo::new(
            TypeKind::Enum,
            "ButtonCopy",
            &[],
            "src/ui/button.rs",
            "demo",
            Some(NamespaceRule::FileRelative),
        );

        let namespace = info
            .try_resolved_namespace(&manifest_dir)
            .expect("namespace should resolve")
            .expect("namespace configured");

        assert_eq!(namespace.as_str(), "ui/button");
        assert_eq!(
            namespace.try_resource_key("demo").unwrap().as_str(),
            "demo/ui/button"
        );
    }

    #[test]
    fn ftl_type_info_try_resolved_namespace_rejects_invalid_literal() {
        let manifest_dir = test_manifest_dir();
        let info = FtlTypeInfo::new(
            TypeKind::Enum,
            "EscapingCopy",
            &[],
            "src/lib.rs",
            "demo",
            Some(super::__macro::namespace_literal("../escape")),
        );

        let err = info
            .try_resolved_namespace(&manifest_dir)
            .expect_err("invalid literal should fail");

        assert_eq!(err, NamespacePathError::CurrentOrParentSegment);
    }

    #[test]
    fn ftl_type_info_exposes_typed_source_metadata() {
        static VARIANTS: &[FtlVariant] = &[FtlVariant::new(
            "Ready",
            StaticFluentEntryId::new_unchecked("status-Ready"),
            &[],
            "demo",
            42,
        )];
        let info = FtlTypeInfo::new(
            TypeKind::Enum,
            "Status",
            VARIANTS,
            "src/status.rs",
            "demo",
            None,
        );

        assert_eq!(info.source_file().unwrap().as_str(), "src/status.rs");
        assert_eq!(VARIANTS[0].entry_id().as_str(), "status-Ready");
        assert_eq!(
            VARIANTS[0].message_id().expect("message entry").as_str(),
            "status-Ready"
        );
        assert_eq!(VARIANTS[0].argument_names(), Vec::new());
        assert_eq!(VARIANTS[0].source_line().get(), 42);

        let location = info.source_location_for(&VARIANTS[0]).unwrap();
        assert_eq!(location.file().as_str(), "src/status.rs");
        assert_eq!(location.line().get(), 42);
    }

    #[test]
    fn empty_type_file_path_has_no_typed_source_location() {
        static VARIANTS: &[FtlVariant] = &[FtlVariant::new(
            "Ready",
            StaticFluentEntryId::new_unchecked("status-Ready"),
            &[],
            "demo",
            42,
        )];
        let info = FtlTypeInfo::new(TypeKind::Enum, "Status", VARIANTS, "", "demo", None);

        assert!(info.source_file().is_none());
        assert!(info.source_location_for(&VARIANTS[0]).is_none());
    }

    #[test]
    fn static_fluent_wrappers_validate_manual_construction() {
        assert_eq!(
            StaticFluentEntryId::try_new("_invalid")
                .unwrap_err()
                .to_string(),
            "Fluent entry id must start with an ASCII letter"
        );
        assert_eq!(
            StaticFluentEntryId::try_new("-shared-term")
                .expect("term entry")
                .as_str(),
            "-shared-term"
        );
        assert_eq!(
            StaticFluentDomain::try_new("app-domain")
                .expect("domain")
                .as_str(),
            "app-domain"
        );
        assert_eq!(
            StaticFluentDomain::from_package_name("package-domain").as_str(),
            "package-domain"
        );
        assert_eq!(
            StaticFluentArgumentName::try_new("not valid")
                .unwrap_err()
                .to_string(),
            "Fluent argument name contains invalid character ' '; use ASCII letters, digits, '_' or '-'"
        );
    }
}
