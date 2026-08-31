use es_fluent_runner::PackageName;
use fs_err as fs;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Runner binary cache tracking which content hashes it was built with.
///
/// Stored at the workspace level since the runner is monolithic.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RunnerCache {
    /// Map of crate name -> content hash when runner was last built.
    pub crate_hashes: IndexMap<PackageName, String>,
    /// Mtime of runner binary when cache was created
    pub runner_mtime: u64,
    /// Version of es-fluent-cli that built this runner
    /// Missing/mismatched version triggers rebuild to pick up helper changes
    #[serde(default)]
    pub cli_version: String,
    /// Serialized request and metadata contract used by the cached runner.
    #[serde(default)]
    pub runner_protocol_version: u32,
    /// Hash of workspace-level Cargo inputs, including transitive config, manifest, and lockfiles.
    #[serde(default)]
    pub workspace_inputs_hash: String,
}

impl RunnerCache {
    const CACHE_FILE: &'static str = "runner_cache.json";

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
}
