//! Shared resource planning types used by managers and generation tooling.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::CanonicalLanguageIdentifierError;
use crate::LanguageIdentifier;
use crate::namespace::{NamespacePathError, ResolvedNamespace};

/// Errors produced while building a module resource plan.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum ResourcePlanError {
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

/// Error produced while validating a resource key.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
#[error("{0}")]
pub struct ResourceKeyError(#[from] NamespacePathError);

/// Stable key for a localized resource.
///
/// Keys use the canonical shape:
/// - `{domain}` for base files
/// - `{domain}/{namespace}` for namespaced files
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceKey(String);

impl ResourceKey {
    /// Validates and creates a resource key.
    pub fn try_new(key: impl Into<String>) -> Result<Self, ResourceKeyError> {
        let key = key.into();
        crate::namespace::validate_namespace_path_typed(&key)?;
        Ok(Self(key))
    }

    /// Creates a new resource key.
    #[allow(
        clippy::panic,
        reason = "panic wrapper retained for static metadata; use try_new for dynamic input"
    )]
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        Self::try_new(key.clone())
            .unwrap_or_else(|error| panic!("invalid resource key '{key}': {error}"))
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the domain segment of the key.
    pub fn domain(&self) -> &str {
        self.0.split('/').next().unwrap_or(self.as_str())
    }
}

impl AsRef<str> for ResourceKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<String> for ResourceKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ResourceKey {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Locale-relative path to a Fluent resource file.
///
/// Paths use the canonical shape `{domain}.ftl` or
/// `{domain}/{namespace}.ftl` and are relative to a locale root.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocaleRelativeFtlPath(String);

impl LocaleRelativeFtlPath {
    /// Validates and creates a locale-relative Fluent resource path.
    pub fn try_new(path: impl Into<String>) -> Result<Self, LocaleRelativeFtlPathError> {
        let path = path.into();
        validate_locale_relative_ftl_path(&path)?;
        Ok(Self(path))
    }

    /// Creates a locale-relative Fluent resource path, panicking when invalid.
    #[allow(
        clippy::panic,
        reason = "panic wrapper retained for static metadata; use try_new for dynamic input"
    )]
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        Self::try_new(path.clone())
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

impl AsRef<str> for LocaleRelativeFtlPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for LocaleRelativeFtlPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
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

impl fmt::Display for LocaleRelativeFtlPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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

/// Canonical description of a single localized resource file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleResourceSpec {
    /// Stable resource key used by managers (e.g., `my-crate`, `my-crate/ui`, `my-crate/ui/button`).
    pub key: ResourceKey,
    /// Path under a locale root (e.g., `my-crate.ftl`, `my-crate/ui.ftl`, `my-crate/ui/button.ftl`).
    pub locale_relative_path: LocaleRelativeFtlPath,
    /// Whether this resource is required for locale readiness.
    pub required: bool,
}

impl ModuleResourceSpec {
    /// Validates and creates a resource specification.
    pub fn try_new(
        key: impl Into<String>,
        locale_relative_path: impl Into<String>,
        required: bool,
    ) -> Result<Self, ResourcePlanError> {
        let key = key.into();
        let locale_relative_path = locale_relative_path.into();
        Ok(Self {
            key: ResourceKey::try_new(key.clone())
                .map_err(|details| ResourcePlanError::InvalidResourceKey { key, details })?,
            locale_relative_path: LocaleRelativeFtlPath::try_new(locale_relative_path.clone())
                .map_err(|details| ResourcePlanError::InvalidResourcePath {
                    path: locale_relative_path,
                    details,
                })?,
            required,
        })
    }

    /// Creates a resource specification.
    pub fn new(
        key: impl Into<ResourceKey>,
        locale_relative_path: impl Into<String>,
        required: bool,
    ) -> Self {
        Self {
            key: key.into(),
            locale_relative_path: LocaleRelativeFtlPath::new(locale_relative_path),
            required,
        }
    }

    /// Validates and creates the base domain resource specification.
    pub fn try_base(domain: &str, required: bool) -> Result<Self, ResourcePlanError> {
        Self::try_new(domain, format!("{domain}.ftl"), required)
    }

    /// Creates the base domain resource specification.
    pub fn base(domain: &str, required: bool) -> Self {
        Self::new(
            ResourceKey::new(domain.to_string()),
            format!("{domain}.ftl"),
            required,
        )
    }

    /// Validates and creates a namespaced resource specification.
    pub fn try_namespaced(
        domain: &str,
        namespace: &ResolvedNamespace,
        required: bool,
    ) -> Result<Self, ResourcePlanError> {
        Self::try_new(
            format!("{domain}/{namespace}"),
            format!("{domain}/{namespace}.ftl"),
            required,
        )
    }

    /// Creates a namespaced resource specification.
    pub fn namespaced(domain: &str, namespace: &ResolvedNamespace, required: bool) -> Self {
        Self::new(
            namespace.resource_key(domain),
            format!("{domain}/{namespace}.ftl"),
            required,
        )
    }

    /// Returns the full path for a locale (e.g., `en/my-crate.ftl`).
    pub fn locale_path(&self, lang: &LanguageIdentifier) -> String {
        format!("{}/{}", lang, self.locale_relative_path)
    }
}

/// Canonical resource plan for a module domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourcePlan {
    specs: Vec<ModuleResourceSpec>,
}

/// Sparse per-language resource plans discovered from a locale asset tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseAssetResourcePlans {
    languages: Vec<LanguageIdentifier>,
    namespaces: Vec<ResolvedNamespace>,
    resource_specs_by_language: Vec<(LanguageIdentifier, Vec<ModuleResourceSpec>)>,
}

impl SparseAssetResourcePlans {
    /// Returns canonical language identifiers discovered in the assets tree.
    pub fn languages(&self) -> &[LanguageIdentifier] {
        &self.languages
    }

    /// Returns all namespace paths discovered across languages.
    pub fn namespaces(&self) -> &[ResolvedNamespace] {
        &self.namespaces
    }

    /// Returns sparse resource plans keyed by language identifier.
    pub fn resource_specs_by_language(&self) -> &[(LanguageIdentifier, Vec<ModuleResourceSpec>)] {
        &self.resource_specs_by_language
    }

    /// Converts the discovery result into its component vectors.
    pub fn into_parts(
        self,
    ) -> (
        Vec<LanguageIdentifier>,
        Vec<ResolvedNamespace>,
        Vec<(LanguageIdentifier, Vec<ModuleResourceSpec>)>,
    ) {
        (
            self.languages,
            self.namespaces,
            self.resource_specs_by_language,
        )
    }
}

impl ResourcePlan {
    /// Builds the global/default canonical resource plan for a domain.
    pub fn for_domain(domain: &str, namespaces: &[&str]) -> Result<Self, ResourcePlanError> {
        if namespaces.is_empty() {
            return Ok(Self {
                specs: vec![ModuleResourceSpec::try_base(domain, true)?],
            });
        }

        let mut specs = Vec::with_capacity(namespaces.len() + 1);
        specs.push(ModuleResourceSpec::try_base(domain, false)?);

        let mut seen = HashSet::new();
        for namespace in namespaces {
            let namespace = ResolvedNamespace::new(*namespace).map_err(|details| {
                ResourcePlanError::InvalidNamespace {
                    namespace: (*namespace).to_string(),
                    details,
                }
            })?;

            if !seen.insert(namespace.clone()) {
                continue;
            }

            specs.push(ModuleResourceSpec::try_namespaced(
                domain, &namespace, true,
            )?);
        }

        Ok(Self { specs })
    }

    /// Builds a sparse resource plan for a single locale.
    pub fn sparse_for_domain(
        domain: &str,
        has_base_resource: bool,
        namespaces: &[ResolvedNamespace],
        base_required: bool,
    ) -> Self {
        let mut specs = Vec::with_capacity(namespaces.len() + usize::from(has_base_resource));

        if has_base_resource {
            specs.push(ModuleResourceSpec::base(domain, base_required));
        }

        for namespace in namespaces {
            specs.push(ModuleResourceSpec::namespaced(domain, namespace, true));
        }

        Self { specs }
    }

    /// Discovers sparse per-language resource plans from an assets tree.
    ///
    /// `assets_root` must contain locale directories such as `en-US/`. Within
    /// each locale, `{domain}.ftl` is the base resource and
    /// `{domain}/{namespace}.ftl` entries are namespaced resources.
    pub fn sparse_from_assets(
        domain: &str,
        assets_root: &Path,
    ) -> Result<SparseAssetResourcePlans, SparseAssetResourcePlanError> {
        let entries = std::fs::read_dir(assets_root).map_err(|source| {
            SparseAssetResourcePlanError::ReadAssetsRoot {
                path: assets_root.to_path_buf(),
                source,
            }
        })?;

        let mut namespaces = BTreeSet::new();
        let mut languages_with_base_file = BTreeSet::new();
        let mut discovered_languages = BTreeSet::new();
        let mut namespaces_by_language: BTreeMap<LanguageIdentifier, BTreeSet<ResolvedNamespace>> =
            BTreeMap::new();

        for entry in entries {
            let entry =
                entry.map_err(|source| SparseAssetResourcePlanError::ReadAssetsRootEntry {
                    path: assets_root.to_path_buf(),
                    source,
                })?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let raw_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| SparseAssetResourcePlanError::NonUtf8LocaleDirectory {
                    path: path.clone(),
                })?;
            let canonical_lang =
                crate::parse_canonical_language_identifier(raw_name).map_err(|details| {
                    SparseAssetResourcePlanError::InvalidLocaleDirectory {
                        raw_name: raw_name.to_string(),
                        path: path.clone(),
                        details,
                    }
                })?;

            let base_path = path.join(format!("{domain}.ftl"));
            let namespace_root = path.join(domain);
            let has_base_file = base_path.exists();
            let discovered_namespaces = if namespace_root.is_dir() {
                discover_namespaces(domain, &namespace_root)?
            } else {
                BTreeSet::new()
            };

            if has_base_file || !discovered_namespaces.is_empty() {
                discovered_languages.insert(canonical_lang.clone());
            }
            if has_base_file {
                languages_with_base_file.insert(canonical_lang.clone());
            }
            for namespace in discovered_namespaces {
                namespaces.insert(namespace.clone());
                namespaces_by_language
                    .entry(canonical_lang.clone())
                    .or_default()
                    .insert(namespace);
            }
        }

        let namespaces: Vec<ResolvedNamespace> = namespaces.into_iter().collect();
        let languages: Vec<LanguageIdentifier> = discovered_languages.into_iter().collect();
        let mut resource_specs_by_language = Vec::with_capacity(languages.len());

        for lang in &languages {
            if namespaces.is_empty() {
                let plan = Self::sparse_for_domain(domain, true, &[], true);
                resource_specs_by_language.push((lang.clone(), plan.into_specs()));
                continue;
            }

            let resolved_namespaces = namespaces_by_language
                .get(lang)
                .into_iter()
                .flat_map(|entries| entries.iter())
                .cloned()
                .collect::<Vec<_>>();

            let plan = Self::sparse_for_domain(
                domain,
                languages_with_base_file.contains(lang),
                &resolved_namespaces,
                false,
            );
            resource_specs_by_language.push((lang.clone(), plan.into_specs()));
        }

        Ok(SparseAssetResourcePlans {
            languages,
            namespaces,
            resource_specs_by_language,
        })
    }

    /// Returns the resource specs in this plan.
    pub fn specs(&self) -> &[ModuleResourceSpec] {
        &self.specs
    }

    /// Converts the plan into its resource specs.
    pub fn into_specs(self) -> Vec<ModuleResourceSpec> {
        self.specs
    }
}

fn namespace_from_relative_ftl_path(
    domain: &str,
    namespace_root: &Path,
    path: &Path,
) -> Result<Option<ResolvedNamespace>, SparseAssetResourcePlanError> {
    if !path.is_file() {
        return Ok(None);
    }

    if path.extension().and_then(|ext| ext.to_str()) != Some("ftl") {
        return Ok(None);
    }

    let relative_path = path.strip_prefix(namespace_root).map_err(|source| {
        SparseAssetResourcePlanError::NamespaceRelativePath {
            path: path.to_path_buf(),
            root: namespace_root.to_path_buf(),
            source,
        }
    })?;
    let relative_without_extension = relative_path.with_extension("");
    let mut components = Vec::new();

    for component in relative_without_extension.components() {
        let value = component.as_os_str().to_str().ok_or_else(|| {
            SparseAssetResourcePlanError::NonUtf8NamespacePath {
                path: relative_without_extension.clone(),
            }
        })?;
        components.push(value.to_string());
    }

    if components.is_empty() {
        return Ok(None);
    }

    let namespace = components.join("/");
    ResolvedNamespace::new(namespace.clone())
        .map(Some)
        .map_err(|details| SparseAssetResourcePlanError::InvalidNamespace {
            namespace,
            domain: domain.to_string(),
            details,
        })
}

fn discover_namespaces(
    domain: &str,
    namespace_root: &Path,
) -> Result<BTreeSet<ResolvedNamespace>, SparseAssetResourcePlanError> {
    let mut namespaces = BTreeSet::new();
    let mut pending = vec![namespace_root.to_path_buf()];

    while let Some(current_dir) = pending.pop() {
        let entries = std::fs::read_dir(&current_dir).map_err(|source| {
            SparseAssetResourcePlanError::ReadNamespaceDirectory {
                path: current_dir.clone(),
                source,
            }
        })?;

        for entry in entries {
            let entry = entry.map_err(|source| {
                SparseAssetResourcePlanError::ReadNamespaceDirectoryEntry {
                    path: current_dir.clone(),
                    source,
                }
            })?;
            let path = entry.path();

            if path.is_dir() {
                pending.push(path);
                continue;
            }

            if let Some(namespace) =
                namespace_from_relative_ftl_path(domain, namespace_root, &path)?
            {
                namespaces.insert(namespace);
            }
        }
    }

    Ok(namespaces)
}

impl From<ResourcePlan> for Vec<ModuleResourceSpec> {
    fn from(plan: ResourcePlan) -> Self {
        plan.into_specs()
    }
}

/// Builds the global/default canonical resource plan for a domain.
///
/// Contract:
/// - Without namespaces, `{domain}.ftl` is required.
/// - With namespaces, `{domain}.ftl` remains an optional mixed-mode resource
///   and `{domain}/{namespace}.ftl` entries are required.
/// - Compile-time registrations may provide a sparse per-language plan through
///   `I18nModuleRegistration::resource_plan_for_language`; managers should use
///   that plan directly when it is available.
pub fn try_resource_plan_for(
    domain: &str,
    namespaces: &[&str],
) -> Result<Vec<ModuleResourceSpec>, ResourcePlanError> {
    ResourcePlan::for_domain(domain, namespaces).map(ResourcePlan::into_specs)
}

/// Builds the global/default canonical resource plan for a domain.
///
/// Prefer [`try_resource_plan_for`] when invalid namespace input should be
/// reported as data instead of aborting. This panic wrapper is retained for
/// static module metadata paths where invalid namespaces are programmer errors
/// and recovery is not useful.
#[allow(
    clippy::panic,
    reason = "panic wrapper retained for static metadata; use try_resource_plan_for for dynamic input"
)]
pub fn resource_plan_for(domain: &str, namespaces: &[&str]) -> Vec<ModuleResourceSpec> {
    try_resource_plan_for(domain, namespaces)
        .unwrap_or_else(|error| panic!("resource_plan_for received {error}"))
}

/// Returns required resource keys from a resource plan.
pub fn required_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<ResourceKey> {
    plan.iter()
        .filter(|spec| spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns optional resource keys from a resource plan.
pub fn optional_resource_keys_from_plan(plan: &[ModuleResourceSpec]) -> HashSet<ResourceKey> {
    plan.iter()
        .filter(|spec| !spec.required)
        .map(|spec| spec.key.clone())
        .collect()
}

/// Returns true when all required keys are present in the loaded set.
pub fn locale_is_ready(
    required_keys: &HashSet<ResourceKey>,
    loaded_keys: &HashSet<ResourceKey>,
) -> bool {
    required_keys.iter().all(|key| loaded_keys.contains(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace::NamespacePathError;

    #[test]
    fn resource_key_conversions_preserve_key_and_domain() {
        let from_string: ResourceKey = "demo/ui".to_string().into();
        let from_str: ResourceKey = "demo/errors".into();

        assert_eq!(from_string.as_str(), "demo/ui");
        assert_eq!(from_string.domain(), "demo");
        assert_eq!(from_string.as_ref(), "demo/ui");
        assert_eq!(from_str.to_string(), "demo/errors");
    }

    #[test]
    fn resource_key_rejects_noncanonical_key_shapes() {
        assert_eq!(
            ResourceKey::try_new("../demo").expect_err("parent segment"),
            ResourceKeyError(NamespacePathError::CurrentOrParentSegment)
        );
        assert_eq!(
            ResourceKey::try_new("demo.ftl").expect_err("file suffix"),
            ResourceKeyError(NamespacePathError::FileExtension)
        );
        assert_eq!(
            ResourceKey::try_new("demo//ui").expect_err("empty segment"),
            ResourceKeyError(NamespacePathError::EmptySegment)
        );
    }

    #[test]
    fn locale_relative_ftl_path_validates_canonical_resource_paths() {
        let path = LocaleRelativeFtlPath::try_new("demo/ui.ftl").expect("path");
        assert_eq!(path.as_str(), "demo/ui.ftl");
        assert_eq!(path.to_string(), "demo/ui.ftl");
        assert_eq!(path, "demo/ui.ftl");

        assert_eq!(
            LocaleRelativeFtlPath::try_new("").expect_err("empty path"),
            LocaleRelativeFtlPathError::Empty
        );
        assert_eq!(
            LocaleRelativeFtlPath::try_new("/demo.ftl").expect_err("absolute path"),
            LocaleRelativeFtlPathError::Absolute
        );
        assert_eq!(
            LocaleRelativeFtlPath::try_new("demo\\ui.ftl").expect_err("backslash path"),
            LocaleRelativeFtlPathError::Backslash
        );
        assert_eq!(
            LocaleRelativeFtlPath::try_new("demo/ui").expect_err("missing suffix"),
            LocaleRelativeFtlPathError::MissingFtlSuffix
        );
        assert!(matches!(
            LocaleRelativeFtlPath::try_new("demo/../ui.ftl"),
            Err(LocaleRelativeFtlPathError::InvalidStem(
                NamespacePathError::CurrentOrParentSegment
            ))
        ));
    }

    #[test]
    fn module_resource_spec_try_new_reports_invalid_parts() {
        assert!(matches!(
            ModuleResourceSpec::try_new("../demo", "demo.ftl", true),
            Err(ResourcePlanError::InvalidResourceKey {
                key,
                details: ResourceKeyError(NamespacePathError::CurrentOrParentSegment),
            }) if key == "../demo"
        ));
        assert!(matches!(
            ModuleResourceSpec::try_new("demo", "demo", true),
            Err(ResourcePlanError::InvalidResourcePath {
                path,
                details: LocaleRelativeFtlPathError::MissingFtlSuffix,
            }) if path == "demo"
        ));
    }

    #[test]
    fn resource_plan_for_handles_base_and_namespaced_resources() {
        let base_plan = resource_plan_for("demo", &[]);
        assert_eq!(base_plan.len(), 1);
        assert_eq!(base_plan[0].key.as_str(), "demo");
        assert_eq!(base_plan[0].locale_relative_path, "demo.ftl");
        let en_us = "en-US".parse().expect("language id");
        assert_eq!(base_plan[0].locale_path(&en_us), "en-US/demo.ftl");
        assert!(base_plan[0].required);

        let namespaced_plan = resource_plan_for("demo", &["ui", "ui", "errors"]);
        let keys: Vec<_> = namespaced_plan
            .iter()
            .map(|spec| spec.key.as_str())
            .collect();
        assert_eq!(keys, vec!["demo", "demo/ui", "demo/errors"]);
        assert!(!namespaced_plan[0].required);
        assert_eq!(namespaced_plan[0].locale_relative_path, "demo.ftl");
        assert!(namespaced_plan[1].required);
        assert!(
            namespaced_plan
                .iter()
                .all(|spec| spec.locale_relative_path.ends_with(".ftl"))
        );
    }

    #[test]
    fn resource_plan_api_exposes_specs_and_sparse_plans() {
        let plan = ResourcePlan::for_domain("demo", &["ui"]).expect("resource plan");
        assert_eq!(plan.specs()[0], ModuleResourceSpec::base("demo", false));
        assert_eq!(plan.specs()[1].key, ResourceKey::new("demo/ui"));

        let namespace = ResolvedNamespace::new("errors/forms").expect("namespace");
        let sparse = ResourcePlan::sparse_for_domain("demo", true, &[namespace], false);
        let specs = sparse.into_specs();

        assert_eq!(
            specs,
            vec![
                ModuleResourceSpec::base("demo", false),
                ModuleResourceSpec::new("demo/errors/forms", "demo/errors/forms.ftl", true),
            ]
        );
    }

    #[test]
    fn sparse_from_assets_discovers_canonical_language_resource_plans() {
        let temp = tempfile::tempdir().expect("tempdir");
        let assets = temp.path();
        std::fs::create_dir_all(assets.join("en-US/demo")).expect("create en assets");
        std::fs::create_dir_all(assets.join("fr/demo/forms")).expect("create fr assets");
        std::fs::write(assets.join("en-US/demo.ftl"), "hello = Hello").expect("write en base");
        std::fs::write(assets.join("en-US/demo/ui.ftl"), "title = UI").expect("write en ui");
        std::fs::write(assets.join("fr/demo/forms/button.ftl"), "button = Bouton")
            .expect("write fr namespace");
        std::fs::write(assets.join("fr/demo/ignore.txt"), "ignored").expect("write ignored");

        let plans = ResourcePlan::sparse_from_assets("demo", assets).expect("plans");

        assert_eq!(
            plans
                .languages()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["en-US", "fr"]
        );
        assert_eq!(
            plans
                .namespaces()
                .iter()
                .map(ResolvedNamespace::as_str)
                .collect::<Vec<_>>(),
            vec!["forms/button", "ui"]
        );
        let specs_by_language = plans
            .resource_specs_by_language()
            .iter()
            .map(|(language, specs)| (language.to_string(), specs.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            specs_by_language,
            vec![
                (
                    "en-US".to_string(),
                    vec![
                        ModuleResourceSpec::base("demo", false),
                        ModuleResourceSpec::namespaced(
                            "demo",
                            &ResolvedNamespace::new("ui").expect("ui namespace"),
                            true
                        ),
                    ]
                ),
                (
                    "fr".to_string(),
                    vec![ModuleResourceSpec::namespaced(
                        "demo",
                        &ResolvedNamespace::new("forms/button").expect("forms namespace"),
                        true
                    ),]
                ),
            ]
        );
    }

    #[test]
    fn sparse_from_assets_requires_base_when_no_namespaces_exist() {
        let temp = tempfile::tempdir().expect("tempdir");
        let assets = temp.path();
        std::fs::create_dir_all(assets.join("en")).expect("create en assets");
        std::fs::write(assets.join("en/demo.ftl"), "hello = Hello").expect("write en base");

        let plans = ResourcePlan::sparse_from_assets("demo", assets).expect("plans");

        let specs_by_language = plans
            .resource_specs_by_language()
            .iter()
            .map(|(language, specs)| (language.to_string(), specs.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            specs_by_language,
            vec![(
                "en".to_string(),
                vec![ModuleResourceSpec::base("demo", true)]
            )]
        );
    }

    #[test]
    fn sparse_from_assets_rejects_noncanonical_locale_directories() {
        let temp = tempfile::tempdir().expect("tempdir");
        let assets = temp.path();
        std::fs::create_dir_all(assets.join("en-us")).expect("create invalid locale");
        std::fs::write(assets.join("en-us/demo.ftl"), "hello = Hello").expect("write base");

        let error = ResourcePlan::sparse_from_assets("demo", assets).expect_err("invalid locale");

        assert!(matches!(
            error,
            SparseAssetResourcePlanError::InvalidLocaleDirectory { ref raw_name, .. }
                if raw_name == "en-us"
        ));
        assert!(
            error
                .to_string()
                .contains("must use canonical BCP-47 form 'en-US'")
        );
    }

    #[test]
    fn sparse_from_assets_rejects_invalid_namespaces() {
        let temp = tempfile::tempdir().expect("tempdir");
        let assets = temp.path();
        std::fs::create_dir_all(assets.join("en/demo")).expect("create en assets");
        std::fs::write(assets.join("en/demo/bad.ftl.ftl"), "bad = Bad")
            .expect("write invalid namespace");

        let error =
            ResourcePlan::sparse_from_assets("demo", assets).expect_err("invalid namespace");

        assert!(matches!(
            error,
            SparseAssetResourcePlanError::InvalidNamespace {
                namespace,
                domain,
                details: NamespacePathError::FileExtension,
            } if namespace == "bad.ftl" && domain == "demo"
        ));
    }

    #[test]
    #[should_panic(expected = "resource_plan_for received invalid namespace")]
    fn resource_plan_for_rejects_invalid_namespaces() {
        let _ = resource_plan_for("demo", &["../outside"]);
    }

    #[test]
    fn try_resource_plan_for_reports_invalid_namespaces() {
        let err = try_resource_plan_for("demo", &["../outside"])
            .expect_err("invalid namespace should fail");

        assert_eq!(
            err,
            ResourcePlanError::InvalidNamespace {
                namespace: "../outside".to_string(),
                details: NamespacePathError::CurrentOrParentSegment
            }
        );
    }

    #[test]
    fn try_resource_plan_for_reports_invalid_domain_without_panicking() {
        let err = try_resource_plan_for("../demo", &[]).expect_err("invalid domain should fail");

        assert_eq!(
            err,
            ResourcePlanError::InvalidResourceKey {
                key: "../demo".to_string(),
                details: ResourceKeyError(NamespacePathError::CurrentOrParentSegment),
            }
        );
    }

    #[test]
    fn resource_plan_uses_resolved_namespace_keys() {
        let plan = resource_plan_for("demo", &["ui/button"]);

        assert_eq!(plan[1].key, ResourceKey::new("demo/ui/button"));
        assert_eq!(plan[1].locale_relative_path, "demo/ui/button.ftl");
    }

    #[test]
    fn required_and_optional_keys_reflect_plan_membership() {
        let plan = vec![
            ModuleResourceSpec::new("demo", "demo.ftl", true),
            ModuleResourceSpec::new("demo/optional", "demo/optional.ftl", false),
        ];

        let required = required_resource_keys_from_plan(&plan);
        let optional = optional_resource_keys_from_plan(&plan);

        assert!(required.contains(&ResourceKey::from("demo")));
        assert!(!required.contains(&ResourceKey::from("demo/optional")));
        assert!(optional.contains(&ResourceKey::from("demo/optional")));
        assert!(!optional.contains(&ResourceKey::from("demo")));

        let loaded = HashSet::from([ResourceKey::from("demo")]);
        assert!(locale_is_ready(&required, &loaded));

        let unloaded = HashSet::new();
        assert!(!locale_is_ready(&required, &unloaded));
    }
}
