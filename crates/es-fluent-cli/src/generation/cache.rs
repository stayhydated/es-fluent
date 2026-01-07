//! Caching utilities for CLI performance optimization.
//!
//! This module provides caching for expensive operations like:
//! - Cargo metadata results
//! - Runner binary staleness detection via content hashing

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Cache of cargo metadata results.
///
/// Stores extracted dependency info keyed by Cargo.lock hash to avoid
/// running cargo_metadata on every invocation.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct MetadataCache {
    /// Hash of Cargo.lock when cache was created
    pub cargo_lock_hash: String,
    /// Extracted es-fluent dependency string
    pub es_fluent_dep: String,
    /// Extracted es-fluent-cli-helpers dependency string
    pub es_fluent_cli_helpers_dep: String,
    /// Target directory
    pub target_dir: String,
}

impl MetadataCache {
    const CACHE_FILE: &'static str = "metadata_cache.json";

    /// Load cache from the temp directory.
    pub fn load(temp_dir: &Path) -> Option<Self> {
        let cache_path = temp_dir.join(Self::CACHE_FILE);
        let content = std::fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save cache to the temp directory.
    pub fn save(&self, temp_dir: &Path) -> std::io::Result<()> {
        let cache_path = temp_dir.join(Self::CACHE_FILE);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, content)
    }

    /// Compute hash of Cargo.lock file.
    pub fn hash_cargo_lock(workspace_root: &Path) -> Option<String> {
        let lock_path = workspace_root.join("Cargo.lock");
        let content = std::fs::read(&lock_path).ok()?;
        Some(blake3::hash(&content).to_hex().to_string())
    }

    /// Check if the Cargo.lock hash matches the cached one.
    pub fn is_valid(&self, workspace_root: &Path) -> bool {
        Self::hash_cargo_lock(workspace_root)
            .map(|h| h == self.cargo_lock_hash)
            .unwrap_or(false)
    }
}

/// Compute blake3 hash of all .rs files in a source directory.
///
/// Used for staleness detection - saving a file without modifications
/// won't change the hash, avoiding unnecessary rebuilds.
pub fn compute_content_hash(src_dir: &Path) -> String {
    use blake3::Hasher;

    let mut hasher = Hasher::new();
    let mut files: Vec<std::path::PathBuf> = Vec::new();

    if src_dir.exists() {
        let walker = walkdir::WalkDir::new(src_dir);
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
                files.push(path.to_path_buf());
            }
        }
    }

    // Sort for deterministic order
    files.sort();

    // Hash path + content for each file
    for path in files {
        if let Ok(content) = std::fs::read(&path) {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(&content);
        }
    }

    hasher.finalize().to_hex().to_string()
}

/// Runner binary cache tracking which content hashes it was built with.
///
/// Stored at the workspace level since the runner is monolithic.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RunnerCache {
    /// Map of crate name -> content hash when runner was last built
    pub crate_hashes: std::collections::HashMap<String, String>,
    /// Mtime of runner binary when cache was created
    pub runner_mtime: u64,
    /// Version of es-fluent-cli that built this runner
    /// Missing/mismatched version triggers rebuild to pick up helper changes
    #[serde(default)]
    pub cli_version: String,
}

impl RunnerCache {
    const CACHE_FILE: &'static str = "runner_cache.json";

    /// Load cache from the temp directory.
    pub fn load(temp_dir: &Path) -> Option<Self> {
        let cache_path = temp_dir.join(Self::CACHE_FILE);
        let content = std::fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save cache to the temp directory.
    pub fn save(&self, temp_dir: &Path) -> std::io::Result<()> {
        let cache_path = temp_dir.join(Self::CACHE_FILE);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, content)
    }
}
