use super::CLI_VERSION;
use cargo_manifest::{Dependency, DependencyDetail};
use es_fluent_runner::RunnerMetadataStore;
use std::{env, path::Path};

type ManifestOverrides = toml::map::Map<String, toml::Value>;

/// Configuration derived from cargo metadata for temp crate generation.
pub(super) struct TempCrateConfig {
    pub(super) es_fluent_dep: Dependency,
    pub(super) es_fluent_cli_helpers_dep: Dependency,
    pub(super) target_dir: String,
    pub(super) manifest_overrides: ManifestOverrides,
}

impl TempCrateConfig {
    /// Create config by querying cargo metadata once, or from cache if valid.
    pub(super) fn from_manifest(manifest_path: &Path) -> Self {
        use crate::generation::cache::MetadataCache;

        let target_dir_from_env = std::env::var("CARGO_TARGET_DIR").ok();
        let manifest_overrides = Self::extract_manifest_overrides(manifest_path);

        let workspace_root = manifest_path.parent().unwrap_or(Path::new("."));
        let temp_dir = RunnerMetadataStore::temp_for_workspace(workspace_root);

        if let Some(cache) = MetadataCache::load(temp_dir.base_dir())
            && cache.is_valid(workspace_root)
        {
            return Self {
                es_fluent_dep: cache.es_fluent_dep,
                es_fluent_cli_helpers_dep: cache.es_fluent_cli_helpers_dep,
                target_dir: target_dir_from_env.unwrap_or(cache.target_dir),
                manifest_overrides,
            };
        }

        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(manifest_path)
            .no_deps()
            .exec()
            .ok();

        let (es_fluent_dep, es_fluent_cli_helpers_dep, target_dir) = match metadata {
            Some(ref meta) => {
                let es_fluent = Self::find_local_dep(meta, "es-fluent")
                    .or_else(Self::find_cli_workspace_dep_es_fluent)
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION));
                let helpers = Self::find_local_dep(meta, "es-fluent-cli-helpers")
                    .or_else(Self::find_cli_workspace_dep_helpers)
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION));
                let target = target_dir_from_env
                    .clone()
                    .unwrap_or_else(|| meta.target_directory.to_string());
                (es_fluent, helpers, target)
            },
            None => (
                Self::find_cli_workspace_dep_es_fluent()
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION)),
                Self::find_cli_workspace_dep_helpers()
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION)),
                target_dir_from_env.unwrap_or_else(|| "../target".to_string()),
            ),
        };

        if let Some(cargo_lock_hash) = MetadataCache::hash_cargo_lock(workspace_root) {
            let _ = std::fs::create_dir_all(temp_dir.base_dir());
            let cache = MetadataCache {
                cargo_lock_hash,
                es_fluent_dep: es_fluent_dep.clone(),
                es_fluent_cli_helpers_dep: es_fluent_cli_helpers_dep.clone(),
                target_dir: target_dir.clone(),
            };
            let _ = cache.save(temp_dir.base_dir());
        }

        Self {
            es_fluent_dep,
            es_fluent_cli_helpers_dep,
            target_dir,
            manifest_overrides,
        }
    }

    fn find_local_dep(meta: &cargo_metadata::Metadata, crate_name: &str) -> Option<Dependency> {
        meta.packages
            .iter()
            .find(|p| p.name.as_str() == crate_name && p.source.is_none())
            .map(|pkg| {
                let path = pkg.manifest_path.parent().unwrap();
                Self::path_dep(path.as_std_path())
            })
    }

    fn find_cli_workspace_dep_es_fluent() -> Option<Dependency> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let es_fluent_path = cli_path.parent()?.join("es-fluent");
        if es_fluent_path.join("Cargo.toml").exists() {
            Some(Self::path_dep(&es_fluent_path))
        } else {
            None
        }
    }

    fn find_cli_workspace_dep_helpers() -> Option<Dependency> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let helpers_path = cli_path.parent()?.join("es-fluent-cli-helpers");
        if helpers_path.join("Cargo.toml").exists() {
            Some(Self::path_dep(&helpers_path))
        } else {
            None
        }
    }

    /// Extract top-level `[patch]` and `[replace]` tables from a workspace manifest.
    ///
    /// The runner crate is an isolated workspace root, so it doesn't inherit dependency
    /// overrides from the project's manifest unless we mirror them into the generated
    /// `.es-fluent/Cargo.toml`.
    pub(super) fn extract_manifest_overrides(manifest_path: &Path) -> ManifestOverrides {
        let content = match std::fs::read_to_string(manifest_path) {
            Ok(content) => content,
            Err(_) => return ManifestOverrides::new(),
        };

        let parsed: toml::Value = match toml::from_str(&content) {
            Ok(parsed) => parsed,
            Err(_) => return ManifestOverrides::new(),
        };

        let Some(table) = parsed.as_table() else {
            return ManifestOverrides::new();
        };

        let mut overrides = ManifestOverrides::new();

        if let Some(patch) = table.get("patch") {
            overrides.insert("patch".to_string(), patch.clone());
        }

        if let Some(replace) = table.get("replace") {
            overrides.insert("replace".to_string(), replace.clone());
        }

        overrides
    }

    fn path_dep(path: &Path) -> Dependency {
        Dependency::Detailed(DependencyDetail {
            path: Some(path.to_string_lossy().into_owned()),
            ..Default::default()
        })
    }

    fn version_dep(version: &str) -> Dependency {
        Dependency::Simple(version.to_string())
    }
}
