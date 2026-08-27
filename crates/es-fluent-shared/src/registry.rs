//! This module provides types for representing FTL variants and type information.

use crate::fluent::{
    FluentArgumentName, FluentDomain, FluentEntryId, FluentIdentifierError, FluentMessageId,
    FluentVariantKey,
};
use crate::meta::TypeKind;
pub use crate::namespace::{NamespacePathError, NamespaceRule, ResolvedNamespace};
use crate::source::{SourceFile, SourceLine, SourceLocation};
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::path::Path;

/// Static Fluent domain emitted by derive macros.
#[derive(derive_more::AsRef, Clone, Copy, Debug, derive_more::Display, Eq, Hash, PartialEq)]
#[as_ref(str)]
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

    pub fn domain_name(self) -> FluentDomain {
        FluentDomain::from_valid_static(self.0)
    }
}

impl Borrow<str> for StaticFluentDomain {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl PartialEq<&str> for StaticFluentDomain {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Static Fluent message identifier emitted by derive macros.
#[derive(derive_more::AsRef, Clone, Copy, Debug, derive_more::Display, Eq, Hash, PartialEq)]
#[as_ref(str)]
pub struct StaticFluentEntryId(&'static str);

impl StaticFluentEntryId {
    /// Creates a static message id from a caller-validated value.
    ///
    /// Derive macros emit this only after validating the message id during macro
    /// expansion. Manual callers should prefer [`Self::try_new`].
    pub(crate) const fn new_unchecked(value: &'static str) -> Self {
        Self(value)
    }

    /// Validates and creates a static message id.
    pub fn try_new(value: &'static str) -> Result<Self, FluentIdentifierError> {
        FluentMessageId::try_new(value)?;
        Ok(Self(value))
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }

    pub fn message_id(self) -> FluentMessageId {
        FluentMessageId::from_valid_static(self.0)
    }

    pub fn entry_id(self) -> FluentEntryId {
        FluentEntryId::from_valid_static(self.0)
    }
}

impl Borrow<str> for StaticFluentEntryId {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl PartialEq<&str> for StaticFluentEntryId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// A fully scoped Fluent message key emitted by derive macros.
///
/// `owner` identifies the crate whose `i18n.toml` defines `domain`. Domains are
/// package-local, so two crates may both define a domain such as `ui` without
/// sharing runtime lookup scope. Generated fallback text is lookup metadata and
/// does not participate in equality or hashing.
#[derive(Clone, Copy, Debug)]
pub struct StaticFluentMessageKey {
    owner: StaticFluentDomain,
    domain: StaticFluentDomain,
    id: StaticFluentEntryId,
    fallback: Option<&'static str>,
}

impl StaticFluentMessageKey {
    /// Creates a fully scoped key from validated static parts.
    pub const fn new(
        owner: StaticFluentDomain,
        domain: StaticFluentDomain,
        id: StaticFluentEntryId,
    ) -> Self {
        Self {
            owner,
            domain,
            id,
            fallback: None,
        }
    }

    /// Creates a fully scoped key with a generated fallback string.
    pub const fn with_fallback(
        owner: StaticFluentDomain,
        domain: StaticFluentDomain,
        id: StaticFluentEntryId,
        fallback: &'static str,
    ) -> Self {
        Self {
            owner,
            domain,
            id,
            fallback: Some(fallback),
        }
    }

    /// Returns the crate that owns this key's domain definition.
    pub const fn owner(self) -> StaticFluentDomain {
        self.owner
    }

    /// Returns the package-local Fluent domain.
    pub const fn domain(self) -> StaticFluentDomain {
        self.domain
    }

    /// Returns the Fluent message identifier.
    pub const fn id(self) -> StaticFluentEntryId {
        self.id
    }

    /// Returns the generated snake_case fallback string when one is available.
    pub const fn fallback(self) -> Option<&'static str> {
        self.fallback
    }
}

impl PartialEq for StaticFluentMessageKey {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.domain == other.domain && self.id == other.id
    }
}

impl Eq for StaticFluentMessageKey {}

impl Hash for StaticFluentMessageKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
        self.domain.hash(state);
        self.id.hash(state);
    }
}

/// Static Fluent argument name emitted by derive macros.
#[derive(derive_more::AsRef, Clone, Copy, Debug, derive_more::Display, Eq, Hash, PartialEq)]
#[as_ref(str)]
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

impl Borrow<str> for StaticFluentArgumentName {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl PartialEq<&str> for StaticFluentArgumentName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

/// Static Fluent select variant key emitted by derive macros.
#[derive(derive_more::AsRef, Clone, Copy, Debug, derive_more::Display, Eq, Hash, PartialEq)]
#[as_ref(str)]
pub struct StaticFluentVariantKey(&'static str);

impl StaticFluentVariantKey {
    /// Creates a static variant key from a caller-validated value.
    ///
    /// Derive macros emit this only after validating the key during macro
    /// expansion. Manual callers should prefer [`Self::try_new`].
    pub(crate) const fn new_unchecked(value: &'static str) -> Self {
        Self(value)
    }

    /// Validates and creates a static select variant key.
    pub fn try_new(value: &'static str) -> Result<Self, FluentIdentifierError> {
        FluentVariantKey::try_new(value)?;
        Ok(Self(value))
    }

    pub fn as_str(self) -> &'static str {
        self.0
    }

    pub fn variant_key(self) -> FluentVariantKey {
        FluentVariantKey::from_valid_static(self.0)
    }
}

impl Borrow<str> for StaticFluentVariantKey {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl PartialEq<&str> for StaticFluentVariantKey {
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
    pub fn message_id(&self) -> FluentMessageId {
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

/// Package and package-local domain scope for generated FTL inventory.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FtlScope {
    source_package: &'static str,
    domain: Option<StaticFluentDomain>,
}

impl FtlScope {
    /// Creates an inventory scope from a Cargo package and optional explicit domain.
    pub const fn new(source_package: &'static str, domain: Option<StaticFluentDomain>) -> Self {
        Self {
            source_package,
            domain,
        }
    }

    /// Returns the Cargo package that declares the inventory.
    pub const fn source_package(self) -> &'static str {
        self.source_package
    }

    /// Returns the optional explicit package-local output domain.
    pub const fn domain(self) -> Option<StaticFluentDomain> {
        self.domain
    }
}

/// Type information for FTL registration, used by derive macros and the CLI.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FtlTypeInfo {
    type_kind: TypeKind,
    type_name: &'static str,
    variants: &'static [FtlVariant],
    /// The Cargo package and optional explicit output domain.
    scope: FtlScope,
    /// The file path where this type is defined (from `file!()` macro).
    file_path: &'static str,
    /// The module path where this type is defined (from `module_path!()` macro).
    module_path: &'static str,
    /// Optional namespace for FTL file output. If Some, the type will be written to
    /// `{lang}/{domain}/{namespace}.ftl` instead of `{lang}/{domain}.ftl`.
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
        scope: FtlScope,
        file_path: &'static str,
        module_path: &'static str,
        namespace: Option<NamespaceRule>,
    ) -> Self {
        Self {
            type_kind,
            type_name,
            variants,
            scope,
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

    pub fn source_package(&self) -> &'static str {
        self.scope.source_package()
    }

    pub fn file_path(&self) -> &'static str {
        self.file_path
    }

    pub fn module_path(&self) -> &'static str {
        self.module_path
    }

    pub fn domain(&self) -> Option<StaticFluentDomain> {
        self.scope.domain()
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
        FtlScope, FtlTypeInfo, FtlVariant, NamespaceRule, ResolvedNamespace,
        StaticFluentArgumentName, StaticFluentDomain, StaticFluentEntryId, StaticFluentMessageKey,
        StaticFluentVariantKey,
    };
    use crate::meta::TypeKind;

    pub const fn static_domain(value: &'static str) -> StaticFluentDomain {
        StaticFluentDomain::new_unchecked(value)
    }

    pub const fn static_entry_id(value: &'static str) -> StaticFluentEntryId {
        StaticFluentEntryId::new_unchecked(value)
    }

    pub const fn static_message_key(
        owner: &'static str,
        domain: StaticFluentDomain,
        id: StaticFluentEntryId,
    ) -> StaticFluentMessageKey {
        StaticFluentMessageKey::new(StaticFluentDomain::new_unchecked(owner), domain, id)
    }

    pub const fn static_message_key_with_fallback(
        owner: &'static str,
        domain: StaticFluentDomain,
        id: StaticFluentEntryId,
        fallback: &'static str,
    ) -> StaticFluentMessageKey {
        StaticFluentMessageKey::with_fallback(
            StaticFluentDomain::new_unchecked(owner),
            domain,
            id,
            fallback,
        )
    }

    pub const fn static_argument_name(value: &'static str) -> StaticFluentArgumentName {
        StaticFluentArgumentName::new_unchecked(value)
    }

    pub const fn static_variant_key(value: &'static str) -> StaticFluentVariantKey {
        StaticFluentVariantKey::new_unchecked(value)
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
        scope: FtlScope,
        file_path: &'static str,
        module_path: &'static str,
        namespace: Option<NamespaceRule>,
    ) -> FtlTypeInfo {
        FtlTypeInfo::new(
            type_kind,
            type_name,
            variants,
            scope,
            file_path,
            module_path,
            namespace,
        )
    }

    pub const fn ftl_scope(
        source_package: &'static str,
        domain: Option<StaticFluentDomain>,
    ) -> FtlScope {
        FtlScope::new(source_package, domain)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FtlScope, FtlTypeInfo, NamespacePathError, NamespaceRule, StaticFluentArgumentName,
        StaticFluentDomain, StaticFluentEntryId, StaticFluentMessageKey, StaticFluentVariantKey,
    };
    use crate::meta::TypeKind;
    use crate::registry::FtlVariant;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_manifest_dir() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"C:\repo\app")
        } else {
            PathBuf::from("/repo/app")
        }
    }

    #[test]
    fn static_variant_key_validates_and_exposes_typed_value() {
        let key = StaticFluentVariantKey::try_new("very-high").expect("valid variant key");

        assert_eq!(key.as_str(), "very-high");
        assert_eq!(key.variant_key().as_str(), "very-high");
        assert!(StaticFluentVariantKey::try_new("not valid").is_err());
    }

    #[test]
    fn fallback_metadata_does_not_change_static_message_key_identity() {
        let owner = StaticFluentDomain::new_unchecked("demo");
        let domain = StaticFluentDomain::new_unchecked("ui");
        let id = StaticFluentEntryId::new_unchecked("save-button");
        let plain = StaticFluentMessageKey::new(owner, domain, id);
        let fallback = StaticFluentMessageKey::with_fallback(owner, domain, id, "save_button");
        let messages = HashMap::from([(plain, "Save")]);

        assert_eq!(plain, fallback);
        assert_eq!(fallback.fallback(), Some("save_button"));
        assert_eq!(messages.get(&fallback), Some(&"Save"));
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
            FtlScope::new("demo", None),
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
            FtlScope::new("demo", None),
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
            FtlScope::new("demo", None),
            "src/status.rs",
            "demo",
            None,
        );

        assert_eq!(info.source_package(), "demo");
        assert_eq!(info.source_file().unwrap().as_str(), "src/status.rs");
        assert_eq!(VARIANTS[0].entry_id().as_str(), "status-Ready");
        assert_eq!(VARIANTS[0].message_id().as_str(), "status-Ready");
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
        let info = FtlTypeInfo::new(
            TypeKind::Enum,
            "Status",
            VARIANTS,
            FtlScope::new("demo", None),
            "",
            "demo",
            None,
        );

        assert!(info.source_file().is_none());
        assert!(info.source_location_for(&VARIANTS[0]).is_none());
    }

    #[test]
    fn static_fluent_wrappers_validate_manual_construction() {
        assert_eq!(
            StaticFluentEntryId::try_new("_invalid")
                .unwrap_err()
                .to_string(),
            "Fluent message id must start with an ASCII letter"
        );
        assert_eq!(
            StaticFluentEntryId::try_new("-shared-term")
                .unwrap_err()
                .to_string(),
            "Fluent message id must start with an ASCII letter"
        );
        assert_eq!(
            StaticFluentDomain::try_new("app-domain")
                .expect("domain")
                .domain_name()
                .as_str(),
            "app-domain"
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
