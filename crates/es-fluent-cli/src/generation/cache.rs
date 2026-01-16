//! Caching utilities for CLI performance optimization.
//!
//! This module provides caching for expensive operations like:
//! - Cargo metadata results
//! - Runner binary staleness detection via content hashing

use indexmap::IndexMap;
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

/// Compute blake3 hash of all .rs files in a source directory, plus the i18n.toml file.
///
/// Used for staleness detection - saving a file without modifications
/// won't change the hash, avoiding unnecessary rebuilds.
///
/// The `i18n_toml_path` parameter includes the i18n.toml configuration file
/// in the hash, so changes to settings like `fluent_feature` trigger rebuilds.
pub fn compute_content_hash(src_dir: &Path, i18n_toml_path: Option<&Path>) -> String {
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

    // Include i18n.toml if provided and exists
    if let Some(toml_path) = i18n_toml_path {
        if toml_path.is_file() {
            if let Ok(content) = std::fs::read(toml_path) {
                hasher.update(toml_path.to_string_lossy().as_bytes());
                hasher.update(&content);
            }
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
    pub crate_hashes: IndexMap<String, String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_compute_content_hash_without_i18n_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let hash1 = compute_content_hash(&src_dir, None);
        let hash2 = compute_content_hash(&src_dir, None);

        // Same content should produce same hash
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_compute_content_hash_with_i18n_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let i18n_path = temp_dir.path().join("i18n.toml");
        fs::write(&i18n_path, "default_language = \"en\"").unwrap();

        let hash_with_toml = compute_content_hash(&src_dir, Some(&i18n_path));
        let hash_without_toml = compute_content_hash(&src_dir, None);

        // Hash should differ when i18n.toml is included
        assert_ne!(hash_with_toml, hash_without_toml);
    }

    #[test]
    fn test_compute_content_hash_changes_when_i18n_toml_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let i18n_path = temp_dir.path().join("i18n.toml");
        fs::write(&i18n_path, "default_language = \"en\"").unwrap();

        let hash1 = compute_content_hash(&src_dir, Some(&i18n_path));

        // Change the i18n.toml content (e.g., changing fluent_feature)
        fs::write(
            &i18n_path,
            "default_language = \"en\"\nfluent_feature = \"i18n\"",
        )
        .unwrap();

        let hash2 = compute_content_hash(&src_dir, Some(&i18n_path));

        // Hash should change when i18n.toml content changes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_unchanged_when_rs_unchanged() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let i18n_path = temp_dir.path().join("i18n.toml");
        fs::write(&i18n_path, "default_language = \"en\"").unwrap();

        let hash1 = compute_content_hash(&src_dir, Some(&i18n_path));

        // Re-write same content (simulates save without changes)
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();
        fs::write(&i18n_path, "default_language = \"en\"").unwrap();

        let hash2 = compute_content_hash(&src_dir, Some(&i18n_path));

        // Hash should remain the same when content is identical
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_content_hash_nonexistent_i18n_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let nonexistent_path = temp_dir.path().join("nonexistent.toml");

        // Should not panic and should produce same hash as None
        let hash_with_nonexistent = compute_content_hash(&src_dir, Some(&nonexistent_path));
        let hash_without = compute_content_hash(&src_dir, None);

        assert_eq!(hash_with_nonexistent, hash_without);
    }

    #[test]
    fn test_compute_content_hash_only_rs_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let hash1 = compute_content_hash(&src_dir, None);

        // Add a non-.rs file - should not affect hash
        fs::write(src_dir.join("notes.txt"), "some notes").unwrap();

        let hash2 = compute_content_hash(&src_dir, None);

        assert_eq!(hash1, hash2);
    }
}
