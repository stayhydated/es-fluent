use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::LanguageIdentifier;
use crate::namespace::ResolvedNamespace;

use super::{ModuleResourceSpec, ResourcePlan, SparseAssetResourcePlanError};

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
