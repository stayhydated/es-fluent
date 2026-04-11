use super::CLI_VERSION;
use es_fluent_derive_core::get_es_fluent_temp_dir;
use std::{env, path::Path};

/// Configuration derived from cargo metadata for temp crate generation.
pub(super) struct TempCrateConfig {
    pub(super) es_fluent_dep: String,
    pub(super) es_fluent_cli_helpers_dep: String,
    pub(super) target_dir: String,
    pub(super) manifest_overrides: String,
}

impl TempCrateConfig {
    /// Create config by querying cargo metadata once, or from cache if valid.
    pub(super) fn from_manifest(manifest_path: &Path) -> Self {
        use crate::generation::cache::MetadataCache;

        let target_dir_from_env = std::env::var("CARGO_TARGET_DIR").ok();
        let manifest_overrides = Self::extract_manifest_overrides(manifest_path);

        let workspace_root = manifest_path.parent().unwrap_or(Path::new("."));
        let temp_dir = get_es_fluent_temp_dir(workspace_root);

        if let Some(cache) = MetadataCache::load(&temp_dir)
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
                    .unwrap_or_else(|| format!(r#"es-fluent = {{ version = "{}" }}"#, CLI_VERSION));
                let helpers = Self::find_local_dep(meta, "es-fluent-cli-helpers")
                    .or_else(Self::find_cli_workspace_dep_helpers)
                    .unwrap_or_else(|| {
                        format!(
                            r#"es-fluent-cli-helpers = {{ version = "{}" }}"#,
                            CLI_VERSION
                        )
                    });
                let target = target_dir_from_env
                    .clone()
                    .unwrap_or_else(|| meta.target_directory.to_string());
                (es_fluent, helpers, target)
            },
            None => (
                Self::find_cli_workspace_dep_es_fluent()
                    .unwrap_or_else(|| format!(r#"es-fluent = {{ version = "{}" }}"#, CLI_VERSION)),
                Self::find_cli_workspace_dep_helpers().unwrap_or_else(|| {
                    format!(
                        r#"es-fluent-cli-helpers = {{ version = "{}" }}"#,
                        CLI_VERSION
                    )
                }),
                target_dir_from_env
                    .clone()
                    .unwrap_or_else(|| "../target".to_string()),
            ),
        };

        if let Some(cargo_lock_hash) = MetadataCache::hash_cargo_lock(workspace_root) {
            let _ = std::fs::create_dir_all(&temp_dir);
            let cache = MetadataCache {
                cargo_lock_hash,
                es_fluent_dep: es_fluent_dep.clone(),
                es_fluent_cli_helpers_dep: es_fluent_cli_helpers_dep.clone(),
                target_dir: target_dir.clone(),
            };
            let _ = cache.save(&temp_dir);
        }

        Self {
            es_fluent_dep,
            es_fluent_cli_helpers_dep,
            target_dir,
            manifest_overrides,
        }
    }

    fn find_local_dep(meta: &cargo_metadata::Metadata, crate_name: &str) -> Option<String> {
        meta.packages
            .iter()
            .find(|p| p.name.as_str() == crate_name && p.source.is_none())
            .map(|pkg| {
                let path = pkg.manifest_path.parent().unwrap();
                format!(r#"{} = {{ path = "{}" }}"#, crate_name, path)
            })
    }

    fn find_cli_workspace_dep_es_fluent() -> Option<String> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let es_fluent_path = cli_path.parent()?.join("es-fluent");
        if es_fluent_path.join("Cargo.toml").exists() {
            Some(format!(
                r#"es-fluent = {{ path = "{}" }}"#,
                es_fluent_path.display()
            ))
        } else {
            None
        }
    }

    fn find_cli_workspace_dep_helpers() -> Option<String> {
        let cli_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cli_path = Path::new(cli_manifest_dir);
        let helpers_path = cli_path.parent()?.join("es-fluent-cli-helpers");
        if helpers_path.join("Cargo.toml").exists() {
            Some(format!(
                r#"es-fluent-cli-helpers = {{ path = "{}" }}"#,
                helpers_path.display()
            ))
        } else {
            None
        }
    }

    /// Extract top-level `[patch]` and `[replace]` tables from a workspace manifest.
    ///
    /// The runner crate is an isolated workspace root, so it doesn't inherit dependency
    /// overrides from the project's manifest unless we mirror them into the generated
    /// `.es-fluent/Cargo.toml`.
    pub(super) fn extract_manifest_overrides(manifest_path: &Path) -> String {
        let content = match std::fs::read_to_string(manifest_path) {
            Ok(content) => content,
            Err(_) => return String::new(),
        };

        let parsed: toml::Value = match toml::from_str(&content) {
            Ok(parsed) => parsed,
            Err(_) => return String::new(),
        };

        let Some(table) = parsed.as_table() else {
            return String::new();
        };

        let mut rendered_sections = Vec::new();

        if let Some(patch) = table.get("patch") {
            let mut patch_table = toml::map::Map::new();
            patch_table.insert("patch".to_string(), patch.clone());
            if let Ok(rendered) = toml::to_string(&toml::Value::Table(patch_table)) {
                rendered_sections.push(rendered.trim_end().to_string());
            }
        }

        if let Some(replace) = table.get("replace") {
            let mut replace_table = toml::map::Map::new();
            replace_table.insert("replace".to_string(), replace.clone());
            if let Ok(rendered) = toml::to_string(&toml::Value::Table(replace_table)) {
                rendered_sections.push(rendered.trim_end().to_string());
            }
        }

        if rendered_sections.is_empty() {
            String::new()
        } else {
            format!("{}\n", rendered_sections.join("\n\n"))
        }
    }
}
