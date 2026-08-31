use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Cache of cargo metadata results.
///
/// Stores extracted dependency info keyed by Cargo.lock hash to avoid
/// running cargo_metadata on every invocation.
#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataCache {
    /// Hash of Cargo.lock when cache was created
    pub cargo_lock_hash: String,
    /// Extracted es-fluent dependency spec
    pub es_fluent_dep: cargo_manifest::Dependency,
    /// Extracted es-fluent-cli-helpers dependency spec
    pub es_fluent_cli_helpers_dep: cargo_manifest::Dependency,
}

impl MetadataCache {
    pub(super) const CACHE_FILE: &'static str = "metadata_cache.json";

    /// Load cache from the temp directory.
    pub fn load(temp_dir: &Path) -> Option<Self> {
        let cache_path = temp_dir.join(Self::CACHE_FILE);
        let content = fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save cache to the temp directory.
    pub fn save(&self, temp_dir: &Path) -> std::io::Result<()> {
        let cache_path = temp_dir.join(Self::CACHE_FILE);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, content)
    }

    /// Compute hash of Cargo.lock file.
    pub fn hash_cargo_lock(workspace_root: &Path) -> Option<String> {
        let lock_path = workspace_root.join("Cargo.lock");
        let content = fs::read(&lock_path).ok()?;
        Some(blake3::hash(&content).to_hex().to_string())
    }

    /// Check if the Cargo.lock hash matches the cached one.
    pub fn is_valid(&self, workspace_root: &Path) -> bool {
        Self::hash_cargo_lock(workspace_root)
            .map(|h| h == self.cargo_lock_hash)
            .unwrap_or(false)
    }
}
