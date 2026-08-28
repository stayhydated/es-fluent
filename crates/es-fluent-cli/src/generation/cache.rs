//! Caching utilities for CLI performance optimization.
//!
//! This module provides caching for expensive operations like:
//! - Cargo metadata results
//! - Runner binary staleness detection via content hashing

use es_fluent_runner::PackageName;
use fs_err as fs;
use indexmap::IndexMap;
use path_slash::PathExt as _;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

const GENERATED_ROOT_SOURCE_DIRS: &[&str] = &[".es-fluent", "target"];

#[derive(Debug)]
pub(crate) struct CargoInputs {
    pub(crate) config_paths: BTreeSet<PathBuf>,
    pub(crate) lockfile_paths: BTreeSet<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct CargoConfigFile {
    #[serde(default)]
    include: Vec<CargoConfigInclude>,
    #[serde(default)]
    resolver: CargoResolverConfig,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CargoConfigInclude {
    Path(PathBuf),
    Detailed { path: PathBuf },
}

impl CargoConfigInclude {
    fn into_path(self) -> PathBuf {
        match self {
            Self::Path(path) | Self::Detailed { path } => path,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct CargoResolverConfig {
    #[serde(rename = "lockfile-path")]
    lockfile_path: Option<PathBuf>,
}

pub(crate) fn configured_cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
}

fn configured_lockfile_path() -> Option<PathBuf> {
    std::env::var_os("CARGO_RESOLVER_LOCKFILE_PATH")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

pub(crate) fn cargo_inputs(workspace_root: &Path, cargo_home: Option<PathBuf>) -> CargoInputs {
    let mut config_dirs = workspace_root
        .ancestors()
        .map(|ancestor| ancestor.join(".cargo"))
        .collect::<BTreeSet<_>>();
    if let Some(cargo_home) = cargo_home {
        config_dirs.insert(if cargo_home.is_absolute() {
            cargo_home
        } else {
            workspace_root.join(cargo_home)
        });
    }

    let mut pending_config_paths = config_dirs
        .iter()
        .flat_map(|directory| [directory.join("config.toml"), directory.join("config")])
        .collect::<Vec<_>>();
    let mut config_paths = BTreeSet::new();
    let mut lockfile_paths = BTreeSet::from([workspace_root.join("Cargo.lock")]);

    if let Some(lockfile_path) = configured_lockfile_path() {
        if lockfile_path.is_absolute() {
            lockfile_paths.insert(normalize_lexical_path(&lockfile_path));
        } else {
            lockfile_paths.insert(resolve_cargo_path(workspace_root, &lockfile_path));
            lockfile_paths.insert(resolve_cargo_path(
                &workspace_root.join(".es-fluent"),
                &lockfile_path,
            ));
            if let Ok(current_dir) = std::env::current_dir() {
                lockfile_paths.insert(resolve_cargo_path(&current_dir, &lockfile_path));
            }
        }
    }

    while let Some(config_path) = pending_config_paths.pop() {
        let config_path = normalize_lexical_path(&config_path);
        if !config_paths.insert(config_path.clone()) {
            continue;
        }

        let Ok(source) = fs::read_to_string(&config_path) else {
            continue;
        };
        let Ok(config) = toml::from_str::<CargoConfigFile>(&source) else {
            continue;
        };
        let config_dir = config_path.parent().unwrap_or(workspace_root);

        pending_config_paths.extend(
            config
                .include
                .into_iter()
                .map(CargoConfigInclude::into_path)
                .map(|path| resolve_cargo_path(config_dir, &path)),
        );
        if let Some(lockfile_path) = config.resolver.lockfile_path {
            let config_value_base = config_dir.parent().unwrap_or(config_dir);
            lockfile_paths.insert(resolve_cargo_path(config_value_base, &lockfile_path));
        }
    }

    CargoInputs {
        config_paths,
        lockfile_paths,
    }
}

fn resolve_cargo_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_lexical_path(path)
    } else {
        normalize_lexical_path(&base.join(path))
    }
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {},
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            },
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    crate::utils::paths::normalize_windows_verbatim_path(&normalized)
}

fn is_ignored_root_source_entry(src_dir: &Path, path: &Path, ignored_root_dirs: &[&str]) -> bool {
    let Ok(relative_path) = path.strip_prefix(src_dir) else {
        return false;
    };

    relative_path
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .is_some_and(|name| ignored_root_dirs.iter().any(|ignored| name == *ignored))
}

fn hash_rs_sources(hasher: &mut blake3::Hasher, src_dir: &Path, ignored_root_dirs: &[&str]) {
    let mut files: Vec<std::path::PathBuf> = Vec::new();

    if src_dir.exists() {
        let walker = walkdir::WalkDir::new(src_dir)
            .into_iter()
            .filter_entry(|entry| {
                !is_ignored_root_source_entry(src_dir, entry.path(), ignored_root_dirs)
            });
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "rs") {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();

    for path in files {
        if let Ok(content) = fs::read(&path) {
            let relative_path = path.strip_prefix(src_dir).unwrap_or(&path);
            let normalized_path = relative_path.to_slash_lossy();
            hasher.update(normalized_path.as_bytes());
            hasher.update(&content);
        }
    }
}

fn hash_optional_file(hasher: &mut blake3::Hasher, label: &str, path: &Path) {
    if let Ok(content) = fs::read(path) {
        hasher.update(label.as_bytes());
        hasher.update(&content);
    }
}

fn hash_reachable_build_sources(
    hasher: &mut blake3::Hasher,
    manifest_dir: &Path,
    build_target_path: &Path,
) -> bool {
    let graph = crate::source_inspector::reachable_source_graph(build_target_path, manifest_dir);
    let cacheable = graph.indeterminate_reasons.is_empty();
    for path in graph.paths {
        let label = path
            .strip_prefix(manifest_dir)
            .unwrap_or(&path)
            .to_slash_lossy();
        hash_optional_file(hasher, &format!("custom-build:{label}"), &path);
    }
    cacheable
}

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
    const CACHE_FILE: &'static str = "metadata_cache.json";

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

/// Compute blake3 hash of crate-local inputs that affect the monolithic runner and watch mode.
///
/// This includes:
/// - `src/**/*.rs`
/// - `i18n.toml` when present
/// - crate-local `Cargo.toml`
/// - the Cargo-selected custom-build target and its reachable local modules
///
/// Returns `None` when the selected custom-build source graph cannot be determined
/// statically, so callers can avoid reusing a potentially stale fingerprint.
pub fn compute_crate_inputs_hash(
    manifest_dir: &Path,
    src_dir: &Path,
    i18n_toml_path: Option<&Path>,
    custom_build_target_path: Option<&Path>,
) -> Option<String> {
    use blake3::Hasher;

    let mut hasher = Hasher::new();
    let ignored_root_dirs = if src_dir == manifest_dir {
        GENERATED_ROOT_SOURCE_DIRS
    } else {
        &[]
    };
    hash_rs_sources(&mut hasher, src_dir, ignored_root_dirs);

    if let Some(toml_path) = i18n_toml_path
        && toml_path.is_file()
    {
        hash_optional_file(&mut hasher, "i18n.toml", toml_path);
    }

    hash_optional_file(&mut hasher, "Cargo.toml", &manifest_dir.join("Cargo.toml"));
    if custom_build_target_path.is_some_and(|build_target_path| {
        !hash_reachable_build_sources(&mut hasher, manifest_dir, build_target_path)
    }) {
        return None;
    }

    Some(hasher.finalize().to_hex().to_string())
}

/// Compute blake3 hash of workspace-level Cargo inputs that affect generation.
///
/// This includes the root manifest, Cargo configuration discovered from the workspace and its
/// ancestors and from the effective Cargo home, recursively included configuration, and every
/// configured lockfile path.
pub fn compute_workspace_inputs_hash(workspace_root: &Path) -> String {
    compute_workspace_inputs_hash_with_cargo_home(workspace_root, configured_cargo_home())
}

pub(crate) fn compute_workspace_inputs_hash_with_cargo_home(
    workspace_root: &Path,
    cargo_home: Option<PathBuf>,
) -> String {
    use blake3::Hasher;

    let mut hasher = Hasher::new();

    hash_optional_file(
        &mut hasher,
        "workspace-manifest",
        &workspace_root.join("Cargo.toml"),
    );

    let cargo_inputs = cargo_inputs(workspace_root, cargo_home);
    for path in cargo_inputs.lockfile_paths {
        hash_framed_path(&mut hasher, "cargo-lockfile", &path);
    }

    for path in cargo_inputs.config_paths {
        hash_framed_path(&mut hasher, "cargo-config", &path);
    }

    hasher.finalize().to_hex().to_string()
}

fn hash_framed_path(hasher: &mut blake3::Hasher, label: &str, path: &Path) {
    hasher.update(label.as_bytes());
    hasher.update(b"\0");
    hasher.update(path.to_slash_lossy().as_bytes());
    hasher.update(b"\0");
    if let Ok(content) = fs::read(path) {
        hasher.update(b"present\0");
        hasher.update(&content);
    } else {
        hasher.update(b"missing");
    }
    hasher.update(b"\0");
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const I18N_CONFIG: &str = "fallback_language = \"en\"\nassets_dir = \"i18n\"\n";

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
        hash_rs_sources(&mut hasher, src_dir, &[]);

        // Include i18n.toml if provided and exists
        if let Some(toml_path) = i18n_toml_path
            && toml_path.is_file()
        {
            hash_optional_file(&mut hasher, "i18n.toml", toml_path);
        }

        hasher.finalize().to_hex().to_string()
    }

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
    fn test_compute_workspace_inputs_hash_changes_when_manifest_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        let first = compute_workspace_inputs_hash(temp_dir.path());
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\nresolver = \"3\"\n",
        )
        .unwrap();
        let second = compute_workspace_inputs_hash(temp_dir.path());

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_workspace_inputs_hash_changes_when_lockfile_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        fs::write(temp_dir.path().join("Cargo.lock"), "version = 4\n").unwrap();

        let first = compute_workspace_inputs_hash(temp_dir.path());
        fs::write(temp_dir.path().join("Cargo.lock"), "version = 5\n").unwrap();
        let second = compute_workspace_inputs_hash(temp_dir.path());

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_workspace_inputs_hash_tracks_ancestor_and_cargo_home_configs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let ancestor = temp_dir.path().join("ancestor");
        let workspace_root = ancestor.join("workspace");
        let cargo_home = temp_dir.path().join("cargo-home");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(
            workspace_root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        let initial = compute_workspace_inputs_hash_with_cargo_home(
            &workspace_root,
            Some(cargo_home.clone()),
        );

        let ancestor_cargo = ancestor.join(".cargo");
        fs::create_dir_all(&ancestor_cargo).unwrap();
        fs::write(
            ancestor_cargo.join("config.toml"),
            "[env]\nINVENTORY_MODE = \"off\"\n",
        )
        .unwrap();
        let with_ancestor = compute_workspace_inputs_hash_with_cargo_home(
            &workspace_root,
            Some(cargo_home.clone()),
        );
        assert_ne!(initial, with_ancestor);

        fs::create_dir_all(&cargo_home).unwrap();
        fs::write(
            cargo_home.join("config"),
            "[build]\nrustflags = [\"--cfg\", \"inventory_on\"]\n",
        )
        .unwrap();
        let with_cargo_home =
            compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home));
        assert_ne!(with_ancestor, with_cargo_home);
    }

    #[test]
    fn test_compute_workspace_inputs_hash_tracks_recursive_configs_and_configured_lockfiles() {
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace_root = temp_dir.path().join("workspace");
        let cargo_home = temp_dir.path().join("cargo-home");
        let cargo_dir = workspace_root.join(".cargo");
        let config_parts = workspace_root.join("config-parts");
        let lock_dir = workspace_root.join("locks");
        fs::create_dir_all(&cargo_dir).unwrap();
        fs::create_dir_all(&config_parts).unwrap();
        fs::create_dir_all(&lock_dir).unwrap();
        fs::write(
            workspace_root.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        fs::write(
            cargo_dir.join("config.toml"),
            concat!(
                "include = [\n",
                "  \"../config-parts/base.toml\",\n",
                "  { path = \"../optional/config.toml\", optional = true },\n",
                "]\n",
            ),
        )
        .unwrap();
        fs::write(
            config_parts.join("base.toml"),
            concat!(
                "include = [\"nested.toml\"]\n",
                "[resolver]\n",
                "lockfile-path = \"locks/Cargo.lock\"\n",
            ),
        )
        .unwrap();
        let nested_config = config_parts.join("nested.toml");
        fs::write(&nested_config, "[env]\nINVENTORY_MODE = \"off\"\n").unwrap();
        let configured_lockfile = lock_dir.join("Cargo.lock");
        fs::write(&configured_lockfile, "version = 4\n").unwrap();

        let initial = compute_workspace_inputs_hash_with_cargo_home(
            &workspace_root,
            Some(cargo_home.clone()),
        );

        fs::write(&nested_config, "[env]\nINVENTORY_MODE = \"on\"\n").unwrap();
        let with_nested_change = compute_workspace_inputs_hash_with_cargo_home(
            &workspace_root,
            Some(cargo_home.clone()),
        );
        assert_ne!(initial, with_nested_change);

        fs::write(&configured_lockfile, "version = 5\n").unwrap();
        let with_lockfile_change = compute_workspace_inputs_hash_with_cargo_home(
            &workspace_root,
            Some(cargo_home.clone()),
        );
        assert_ne!(with_nested_change, with_lockfile_change);

        let optional_config = workspace_root.join("optional/config.toml");
        fs::create_dir_all(optional_config.parent().unwrap()).unwrap();
        fs::write(
            &optional_config,
            "[build]\nrustflags = [\"--cfg\", \"extra\"]\n",
        )
        .unwrap();
        let with_optional_config =
            compute_workspace_inputs_hash_with_cargo_home(&workspace_root, Some(cargo_home));
        assert_ne!(with_lockfile_change, with_optional_config);
    }

    #[test]
    fn test_compute_content_hash_with_i18n_toml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let i18n_path = temp_dir.path().join("i18n.toml");
        fs::write(&i18n_path, I18N_CONFIG).unwrap();

        let hash_with_toml = compute_content_hash(&src_dir, Some(&i18n_path));
        let hash_without_toml = compute_content_hash(&src_dir, None);

        // Hash should differ when i18n.toml is included
        assert_ne!(hash_with_toml, hash_without_toml);
    }

    #[test]
    fn test_compute_crate_inputs_hash_changes_when_crate_manifest_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, None);
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        let second = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, None);

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_crate_inputs_hash_changes_when_build_script_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let build_script = temp_dir.path().join("build.rs");
        let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_script));
        fs::write(&build_script, "fn main() {}\n").unwrap();
        let second =
            compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_script));

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_crate_inputs_hash_tracks_custom_build_modules_but_not_unused_build_rs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        let support_dir = temp_dir.path().join("support");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&support_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
        let build_target = support_dir.join("i18n.rs");
        let helper = support_dir.join("helper.rs");
        fs::write(&build_target, "mod helper; fn main() { helper::run(); }\n").unwrap();
        fs::write(&helper, "pub fn run() {}\n").unwrap();
        fs::write(temp_dir.path().join("build.rs"), "fn main() {}\n").unwrap();

        let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target));
        fs::write(&helper, "pub fn run() { let _changed = true; }\n").unwrap();
        let second =
            compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target));
        assert_ne!(first, second);

        fs::write(
            temp_dir.path().join("build.rs"),
            "fn main() { println!(\"unused\"); }\n",
        )
        .unwrap();
        let third = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target));
        assert_eq!(second, third);
    }

    #[test]
    fn test_compute_crate_inputs_hash_tracks_custom_build_target_outside_package_root() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manifest_dir = temp_dir.path().join("app");
        let src_dir = manifest_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
        let build_target = temp_dir.path().join("shared-build.rs");
        let helper = temp_dir.path().join("shared_helper.rs");
        fs::write(
            &build_target,
            "mod shared_helper; fn main() { shared_helper::run(); }\n",
        )
        .unwrap();
        fs::write(&helper, "pub fn run() {}\n").unwrap();

        let first = compute_crate_inputs_hash(&manifest_dir, &src_dir, None, Some(&build_target))
            .expect("external custom-build graph should be cacheable");
        fs::write(&helper, "pub fn run() { let _changed = true; }\n").unwrap();
        let second = compute_crate_inputs_hash(&manifest_dir, &src_dir, None, Some(&build_target))
            .expect("updated external custom-build graph should be cacheable");

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_crate_inputs_hash_accepts_explicit_path_submodule_layout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        let support_dir = temp_dir.path().join("support");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&support_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
        let build_target = temp_dir.path().join("build.rs");
        fs::write(
            &build_target,
            "#[path = \"support/helper_impl.rs\"] mod assets; fn main() { assets::run(); }\n",
        )
        .unwrap();
        fs::write(
            support_dir.join("helper_impl.rs"),
            "mod nested; pub fn run() { nested::configure(); }\n",
        )
        .unwrap();
        let nested = support_dir.join("nested.rs");
        fs::write(&nested, "pub fn configure() {}\n").unwrap();

        let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
            .expect("explicit-path submodule graph should be cacheable");
        fs::write(&nested, "pub fn configure() { let _changed = true; }\n").unwrap();
        let second =
            compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
                .expect("updated explicit-path submodule graph should be cacheable");

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_crate_inputs_hash_accepts_included_submodule_layout() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        let support_dir = temp_dir.path().join("support");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&support_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
        let build_target = temp_dir.path().join("build.rs");
        fs::write(
            &build_target,
            "include!(\"support/config.rs\"); fn main() { configure(); }\n",
        )
        .unwrap();
        fs::write(
            support_dir.join("config.rs"),
            "mod nested; fn configure() { nested::run(); }\n",
        )
        .unwrap();
        let nested = support_dir.join("nested.rs");
        fs::write(&nested, "pub fn run() {}\n").unwrap();

        let first = compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
            .expect("included submodule graph should be cacheable");
        fs::write(&nested, "pub fn run() { let _changed = true; }\n").unwrap();
        let second =
            compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target))
                .expect("updated included submodule graph should be cacheable");

        assert_ne!(first, second);
    }

    #[test]
    fn test_compute_crate_inputs_hash_is_uncacheable_for_indeterminate_build_graph() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
        let build_target = temp_dir.path().join("build.rs");
        fs::write(
            &build_target,
            "include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/support.rs\"));\nfn main() {}\n",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("support.rs"),
            "pub fn configure() {}\n",
        )
        .unwrap();

        assert_eq!(
            compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target)),
            None
        );
    }

    #[test]
    fn test_compute_crate_inputs_hash_is_uncacheable_for_macro_wrapped_include() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        let support_dir = temp_dir.path().join("support");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&support_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "pub struct App;\n").unwrap();
        let build_target = temp_dir.path().join("build.rs");
        fs::write(
            &build_target,
            "macro_rules! load_config { () => { include!(\"support/config.rs\"); }; } load_config!(); fn main() {}\n",
        )
        .unwrap();
        fs::write(support_dir.join("config.rs"), "pub fn configure() {}\n").unwrap();

        assert_eq!(
            compute_crate_inputs_hash(temp_dir.path(), &src_dir, None, Some(&build_target)),
            None
        );
    }

    #[test]
    fn test_compute_crate_inputs_hash_ignores_generated_dirs_for_root_source_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("lib.rs"), "pub struct Demo;\n").unwrap();

        let first = compute_crate_inputs_hash(temp_dir.path(), temp_dir.path(), None, None);

        fs::create_dir_all(temp_dir.path().join(".es-fluent/src")).unwrap();
        fs::write(
            temp_dir.path().join(".es-fluent/src/main.rs"),
            "fn main() {}\n",
        )
        .unwrap();
        fs::create_dir_all(temp_dir.path().join("target/debug/build/demo/out")).unwrap();
        fs::write(
            temp_dir
                .path()
                .join("target/debug/build/demo/out/generated.rs"),
            "pub fn generated() {}\n",
        )
        .unwrap();

        let second = compute_crate_inputs_hash(temp_dir.path(), temp_dir.path(), None, None);
        assert_eq!(first, second);

        fs::write(temp_dir.path().join("module.rs"), "pub struct Changed;\n").unwrap();
        let third = compute_crate_inputs_hash(temp_dir.path(), temp_dir.path(), None, None);
        assert_ne!(second, third);
    }

    #[test]
    fn test_compute_content_hash_changes_when_i18n_toml_changes() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let i18n_path = temp_dir.path().join("i18n.toml");
        fs::write(&i18n_path, I18N_CONFIG).unwrap();

        let hash1 = compute_content_hash(&src_dir, Some(&i18n_path));

        // Change the i18n.toml content (e.g., changing fluent_feature)
        fs::write(
            &i18n_path,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\nfluent_feature = [\"i18n\"]",
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
        fs::write(&i18n_path, I18N_CONFIG).unwrap();

        let hash1 = compute_content_hash(&src_dir, Some(&i18n_path));

        // Re-write same content (simulates save without changes)
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();
        fs::write(&i18n_path, I18N_CONFIG).unwrap();

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
    fn test_compute_content_hash_ignores_path_aliases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "fn main() {}").unwrap();

        let i18n_path = temp_dir.path().join("i18n.toml");
        fs::write(&i18n_path, I18N_CONFIG).unwrap();

        let aliased_src_dir = temp_dir.path().join("src").join(".");
        let aliased_i18n_path = temp_dir.path().join(".").join("i18n.toml");

        let direct_hash = compute_content_hash(&src_dir, Some(&i18n_path));
        let aliased_hash = compute_content_hash(&aliased_src_dir, Some(&aliased_i18n_path));

        assert_eq!(direct_hash, aliased_hash);
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

    #[test]
    fn metadata_cache_save_load_and_validity_round_trip() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("Cargo.lock"), "lock-content").unwrap();

        let cache = MetadataCache {
            cargo_lock_hash: MetadataCache::hash_cargo_lock(temp_dir.path()).unwrap(),
            es_fluent_dep: cargo_manifest::Dependency::Detailed(cargo_manifest::DependencyDetail {
                path: Some("../es-fluent".to_string()),
                ..Default::default()
            }),
            es_fluent_cli_helpers_dep: cargo_manifest::Dependency::Detailed(
                cargo_manifest::DependencyDetail {
                    path: Some("../helpers".to_string()),
                    ..Default::default()
                },
            ),
        };
        cache.save(temp_dir.path()).unwrap();

        let cache_path = temp_dir.path().join(MetadataCache::CACHE_FILE);
        let mut legacy_cache: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&cache_path).unwrap()).unwrap();
        legacy_cache.as_object_mut().unwrap().insert(
            "target_dir".to_string(),
            serde_json::json!("obsolete-target"),
        );
        fs::write(&cache_path, serde_json::to_vec(&legacy_cache).unwrap()).unwrap();

        let loaded = MetadataCache::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.es_fluent_dep, cache.es_fluent_dep);
        assert!(loaded.is_valid(temp_dir.path()));

        fs::write(temp_dir.path().join("Cargo.lock"), "changed-lock-content").unwrap();
        assert!(!loaded.is_valid(temp_dir.path()));
    }

    #[test]
    fn runner_cache_save_and_load_round_trip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut hashes = IndexMap::new();
        hashes.insert(
            PackageName::try_new("test-crate").expect("valid package name"),
            "abc123".to_string(),
        );

        let cache = RunnerCache {
            crate_hashes: hashes.clone(),
            runner_mtime: 42,
            cli_version: "0.1.0".to_string(),
            runner_protocol_version: es_fluent_runner::RUNNER_PROTOCOL_VERSION,
            workspace_inputs_hash: "workspace-hash".to_string(),
        };
        cache.save(temp_dir.path()).unwrap();

        let loaded = RunnerCache::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.runner_mtime, 42);
        assert_eq!(loaded.cli_version, "0.1.0");
        assert_eq!(
            loaded.runner_protocol_version,
            es_fluent_runner::RUNNER_PROTOCOL_VERSION
        );
        assert_eq!(loaded.crate_hashes, hashes);
        assert_eq!(loaded.workspace_inputs_hash, "workspace-hash");
    }
}
