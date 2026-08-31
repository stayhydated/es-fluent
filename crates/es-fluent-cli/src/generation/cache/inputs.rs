use super::{cargo_inputs, configured_cargo_home};
use fs_err as fs;
use path_slash::PathExt as _;
use std::path::{Component, Path, PathBuf};

const GENERATED_ROOT_SOURCE_DIRS: &[&str] = &[".es-fluent", "target"];

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

pub(super) fn hash_rs_sources(
    hasher: &mut blake3::Hasher,
    src_dir: &Path,
    ignored_root_dirs: &[&str],
) {
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

pub(super) fn hash_optional_file(hasher: &mut blake3::Hasher, label: &str, path: &Path) {
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
