use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A workspace member's place in the member dependency graph.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkspaceMember {
    /// Whether the member's library is a procedural macro, which Cargo
    /// compiles for the host.
    pub proc_macro: bool,
    /// Members this one always depends on as a normal dependency.
    pub deps: Vec<String>,
    /// Members this one always depends on as a build dependency.
    pub build_deps: Vec<String>,
}

/// Cache of cargo metadata results.
///
/// Stores extracted dependency info keyed by the Cargo.lock hash and the
/// workspace manifests' contents to avoid running cargo_metadata on every
/// invocation. It holds only what cargo metadata reports; fallbacks that depend
/// on the running CLI, such as its own version, are resolved on every run.
#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataCache {
    /// Layout version of the cache file; other versions are ignored.
    pub format: u32,
    /// Hash of Cargo.lock when cache was created
    pub cargo_lock_hash: String,
    /// Path dependency on a workspace member named `es-fluent`, if any
    pub local_es_fluent_dep: Option<cargo_manifest::Dependency>,
    /// Path dependency on a workspace member named `es-fluent-cli-helpers`, if any
    pub local_es_fluent_cli_helpers_dep: Option<cargo_manifest::Dependency>,
    /// Workspace members by package name
    pub workspace_members: BTreeMap<String, WorkspaceMember>,
    /// Hash of each workspace manifest the cached metadata was read from.
    ///
    /// Member dependency kinds can change without changing Cargo.lock, for
    /// example when a dependency becomes optional or platform-specific.
    #[serde(default)]
    pub manifest_hashes: BTreeMap<PathBuf, String>,
}

impl MetadataCache {
    pub(super) const CACHE_FILE: &'static str = "metadata_cache.json";
    /// Current [`MetadataCache::format`].
    pub const FORMAT: u32 = 2;

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

    /// Hashes a manifest file, or returns `None` when it cannot be read.
    pub fn hash_manifest(manifest_path: &Path) -> Option<String> {
        let content = fs::read(manifest_path).ok()?;
        Some(blake3::hash(&content).to_hex().to_string())
    }

    /// Check if Cargo.lock and every recorded manifest match the cached hashes.
    pub fn is_valid(&self, workspace_root: &Path) -> bool {
        self.format == Self::FORMAT
            && Self::hash_cargo_lock(workspace_root).is_some_and(|h| h == self.cargo_lock_hash)
            && self
                .manifest_hashes
                .iter()
                .all(|(path, hash)| Self::hash_manifest(path).as_ref() == Some(hash))
    }
}
