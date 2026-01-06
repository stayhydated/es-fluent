//! Caching utilities for CLI performance optimization.
//!
//! This module provides caching for expensive operations like:
//! - Cargo metadata results
//! - Per-crate source file content hashing (for staleness detection)

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

/// Per-crate content hash cache for staleness detection.
///
/// Stored in `metadata/{crate_name}/content_hash.json` alongside inventory.json and result.json.
/// This allows efficient per-crate change detection without hashing all crates.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CrateContentCache {
    /// Blake3 hash of all .rs files in this crate's src directory
    pub content_hash: String,
}

impl CrateContentCache {
    const CACHE_FILE: &'static str = "content_hash.json";

    /// Get the cache directory for a specific crate.
    fn cache_dir(temp_dir: &Path, crate_name: &str) -> std::path::PathBuf {
        temp_dir.join("metadata").join(crate_name)
    }

    /// Load cache for a specific crate.
    pub fn load(temp_dir: &Path, crate_name: &str) -> Option<Self> {
        let cache_path = Self::cache_dir(temp_dir, crate_name).join(Self::CACHE_FILE);
        let content = std::fs::read_to_string(&cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save cache for a specific crate.
    pub fn save(&self, temp_dir: &Path, crate_name: &str) -> std::io::Result<()> {
        let cache_dir = Self::cache_dir(temp_dir, crate_name);
        std::fs::create_dir_all(&cache_dir)?;
        let cache_path = cache_dir.join(Self::CACHE_FILE);
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, content)
    }

    /// Compute blake3 hash of all .rs files in a source directory.
    pub fn compute_hash(src_dir: &Path) -> String {
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

// Keep the old ContentCache for backward compatibility with watcher.rs
// TODO: Migrate watcher.rs to use CrateContentCache directly

/// Compute combined hash of all source files in the given directories.
/// Used by watcher.rs for per-crate change detection.
pub fn compute_content_hash(src_dir: &Path) -> String {
    CrateContentCache::compute_hash(src_dir)
}
