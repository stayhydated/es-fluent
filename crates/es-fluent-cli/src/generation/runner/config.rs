use super::{CLI_VERSION, utf8_path_string};
use anyhow::Result;
use cargo_manifest::{Dependency, DependencyDetail};
use es_fluent_runner::RunnerMetadataStore;
use fs_err as fs;
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
    pub(super) fn from_manifest(manifest_path: &Path) -> Result<Self> {
        use crate::generation::cache::MetadataCache;

        let target_dir_from_env = std::env::var("CARGO_TARGET_DIR").ok();
        let manifest_overrides = Self::extract_manifest_overrides(manifest_path);

        let workspace_root = manifest_path.parent().unwrap_or(Path::new("."));
        let temp_dir = RunnerMetadataStore::temp_for_workspace(workspace_root);

        if let Some(cache) = MetadataCache::load(temp_dir.base_dir())
            && cache.is_valid(workspace_root)
        {
            return Ok(Self {
                es_fluent_dep: cache.es_fluent_dep,
                es_fluent_cli_helpers_dep: cache.es_fluent_cli_helpers_dep,
                target_dir: target_dir_from_env.unwrap_or(cache.target_dir),
                manifest_overrides,
            });
        }

        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(manifest_path)
            .no_deps()
            .exec()
            .ok();

        let (es_fluent_dep, es_fluent_cli_helpers_dep, target_dir) = match metadata {
            Some(ref meta) => {
                let es_fluent = Self::find_local_dep(meta, "es-fluent")?
                    .or(Self::find_cli_workspace_dep_es_fluent()?)
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION));
                let helpers = Self::find_local_dep(meta, "es-fluent-cli-helpers")?
                    .or(Self::find_cli_workspace_dep_helpers()?)
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION));
                let target =
                    target_dir_from_env.unwrap_or_else(|| meta.target_directory.to_string());
                (es_fluent, helpers, target)
            },
            None => (
                Self::find_cli_workspace_dep_es_fluent()?
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION)),
                Self::find_cli_workspace_dep_helpers()?
                    .unwrap_or_else(|| Self::version_dep(CLI_VERSION)),
                target_dir_from_env.unwrap_or_else(|| "../target".to_string()),
            ),
        };

        if let Some(cargo_lock_hash) = MetadataCache::hash_cargo_lock(workspace_root) {
            let _ = fs::create_dir_all(temp_dir.base_dir());
            let cache = MetadataCache {
                cargo_lock_hash,
                es_fluent_dep: es_fluent_dep.clone(),
                es_fluent_cli_helpers_dep: es_fluent_cli_helpers_dep.clone(),
                target_dir: target_dir.clone(),
            };
            let _ = cache.save(temp_dir.base_dir());
        }

        Ok(Self {
            es_fluent_dep,
            es_fluent_cli_helpers_dep,
            target_dir,
            manifest_overrides,
        })
    }

    fn find_local_dep(
        meta: &cargo_metadata::Metadata,
        crate_name: &str,
    ) -> Result<Option<Dependency>> {
        Ok(meta
            .packages
            .iter()
            .find(|p| p.name.as_str() == crate_name && p.source.is_none())
            .map(|pkg| {
                Self::path_dep_utf8(
                    pkg.manifest_path
                        .parent()
                        .expect("manifest path parent")
                        .as_str(),
                )
            }))
    }

    fn find_cli_workspace_dep_es_fluent() -> Result<Option<Dependency>> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let Some(workspace_dir) = cli_path.parent() else {
            return Ok(None);
        };
        let es_fluent_path = workspace_dir.join("es-fluent");
        if es_fluent_path.join("Cargo.toml").exists() {
            Ok(Some(Self::path_dep(
                &es_fluent_path,
                "es-fluent workspace dependency path",
            )?))
        } else {
            Ok(None)
        }
    }

    fn find_cli_workspace_dep_helpers() -> Result<Option<Dependency>> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let Some(workspace_dir) = cli_path.parent() else {
            return Ok(None);
        };
        let helpers_path = workspace_dir.join("es-fluent-cli-helpers");
        if helpers_path.join("Cargo.toml").exists() {
            Ok(Some(Self::path_dep(
                &helpers_path,
                "es-fluent-cli-helpers workspace dependency path",
            )?))
        } else {
            Ok(None)
        }
    }

    /// Extract top-level `[patch]` and `[replace]` tables from a workspace manifest.
    ///
    /// The runner crate is an isolated workspace root, so it doesn't inherit dependency
    /// overrides from the project's manifest unless we mirror them into the generated
    /// `.es-fluent/Cargo.toml`.
    pub(super) fn extract_manifest_overrides(manifest_path: &Path) -> ManifestOverrides {
        let content = match fs::read_to_string(manifest_path) {
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

    fn path_dep(path: &Path, context: &str) -> Result<Dependency> {
        let path = utf8_path_string(path, context)?;
        Ok(Self::path_dep_utf8(&path))
    }

    fn path_dep_utf8(path: &str) -> Dependency {
        Dependency::Detailed(DependencyDetail {
            path: Some(path.to_string()),
            ..Default::default()
        })
    }

    fn version_dep(version: &str) -> Dependency {
        Dependency::Simple(version.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::TempCrateConfig;
    use crate::test_fixtures::{
        toml_helpers::{
            package_manifest as package_manifest_toml, string_value, workspace_manifest, write_toml,
        },
        write_file,
    };
    use cargo_manifest::Dependency;
    use std::path::Path;
    use toml::Value;

    fn dependency_path(dep: Dependency) -> Option<String> {
        match dep {
            Dependency::Detailed(detail) => detail.path,
            _ => None,
        }
    }

    #[test]
    fn find_local_dep_returns_path_dependency_for_local_workspace_package() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_toml(
            &temp.path().join("Cargo.toml"),
            &workspace_manifest(&["app", "es-fluent", "es-fluent-cli-helpers"]),
        );
        write_toml(
            &temp.path().join("app/Cargo.toml"),
            &package_manifest_toml("app", "0.1.0"),
        );
        write_file(&temp.path().join("app/src/lib.rs"), "pub struct App;\n");
        write_toml(
            &temp.path().join("es-fluent/Cargo.toml"),
            &package_manifest_toml("es-fluent", "0.1.0"),
        );
        write_file(
            &temp.path().join("es-fluent/src/lib.rs"),
            "pub struct EsFluent;\n",
        );
        write_toml(
            &temp.path().join("es-fluent-cli-helpers/Cargo.toml"),
            &package_manifest_toml("es-fluent-cli-helpers", "0.1.0"),
        );
        write_file(
            &temp.path().join("es-fluent-cli-helpers/src/lib.rs"),
            "pub struct Helpers;\n",
        );

        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(temp.path().join("Cargo.toml"))
            .no_deps()
            .exec()
            .expect("load cargo metadata");

        let es_fluent =
            TempCrateConfig::find_local_dep(&metadata, "es-fluent").expect("find local dep");
        assert_eq!(
            dependency_path(es_fluent.expect("expected es-fluent dependency")),
            Some(temp.path().join("es-fluent").display().to_string())
        );

        let helpers = TempCrateConfig::find_local_dep(&metadata, "es-fluent-cli-helpers")
            .expect("find helpers dep");
        assert_eq!(
            dependency_path(helpers.expect("expected helpers dependency")),
            Some(
                temp.path()
                    .join("es-fluent-cli-helpers")
                    .display()
                    .to_string()
            )
        );

        assert!(
            TempCrateConfig::find_local_dep(&metadata, "missing-crate")
                .expect("missing crate lookup")
                .is_none()
        );
    }

    #[test]
    fn find_cli_workspace_deps_use_repo_crate_paths() {
        let es_fluent = TempCrateConfig::find_cli_workspace_dep_es_fluent()
            .expect("resolve es-fluent workspace dep");
        assert!(
            dependency_path(es_fluent.expect("expected es-fluent workspace dependency"))
                .is_some_and(|path| path.ends_with("/crates/es-fluent"))
        );

        let helpers = TempCrateConfig::find_cli_workspace_dep_helpers()
            .expect("resolve helpers workspace dep");
        assert!(
            dependency_path(helpers.expect("expected helpers workspace dependency"))
                .is_some_and(|path| path.ends_with("/crates/es-fluent-cli-helpers"))
        );
    }

    #[test]
    fn extract_manifest_overrides_handles_patch_and_invalid_inputs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let patch_manifest = temp.path().join("patch.toml");
        let patch_value: Value = toml::from_str(
            r#"
[patch.crates-io]
serde = { version = "1" }
"#,
        )
        .expect("parse patch manifest fixture");
        write_toml(&patch_manifest, &patch_value);
        let overrides = TempCrateConfig::extract_manifest_overrides(&patch_manifest);
        assert!(overrides.contains_key("patch"));
        assert!(!overrides.contains_key("replace"));

        let invalid_manifest = temp.path().join("invalid.toml");
        write_file(&invalid_manifest, "not = [valid");
        assert!(TempCrateConfig::extract_manifest_overrides(&invalid_manifest).is_empty());

        let non_table_manifest = temp.path().join("scalar.toml");
        write_file(
            &non_table_manifest,
            &string_value("scalar-value").to_string(),
        );
        assert!(TempCrateConfig::extract_manifest_overrides(&non_table_manifest).is_empty());
    }

    #[test]
    fn dependency_builders_preserve_expected_shapes() {
        let path_dep = TempCrateConfig::path_dep(Path::new("/tmp/example"), "example path")
            .expect("build path dependency");
        assert_eq!(dependency_path(path_dep), Some("/tmp/example".to_string()));

        let utf8_dep = TempCrateConfig::path_dep_utf8("/tmp/example-utf8");
        assert_eq!(
            dependency_path(utf8_dep),
            Some("/tmp/example-utf8".to_string())
        );

        match TempCrateConfig::version_dep("1.2.3") {
            Dependency::Simple(version) => assert_eq!(version, "1.2.3"),
            dep => panic!("expected simple version dependency, got {dep:?}"),
        }
    }
}
