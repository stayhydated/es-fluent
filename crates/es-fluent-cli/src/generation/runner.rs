//! Shared functionality for creating and running the runner crate.
//!
//! The CLI uses a monolithic runner crate at workspace root that links ALL workspace
//! crates to access their inventory registrations through a single binary.

use crate::generation::templates::{
    ConfigTomlTemplate, GitignoreTemplate, MonolithicCargoTomlTemplate, MonolithicCrateDep,
    MonolithicMainRsTemplate,
};
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

pub const TEMP_DIR: &str = ".es-fluent";

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Configuration derived from cargo metadata for temp crate generation.
///
/// This calls cargo_metadata once and extracts all needed information:
/// - es-fluent dependency string
/// - es-fluent-cli-helpers dependency string
/// - target directory for sharing compiled dependencies
pub struct TempCrateConfig {
    pub es_fluent_dep: String,
    pub es_fluent_cli_helpers_dep: String,
    pub target_dir: String,
}

impl TempCrateConfig {
    /// Create config by querying cargo metadata once, or from cache if valid.
    pub fn from_manifest(manifest_path: &Path) -> Self {
        use super::cache::MetadataCache;

        // Check CARGO_TARGET_DIR env first (doesn't need metadata)
        let target_dir_from_env = std::env::var("CARGO_TARGET_DIR").ok();

        // Determine workspace root and temp directory for caching
        let workspace_root = manifest_path.parent().unwrap_or(Path::new("."));
        let temp_dir = workspace_root.join(TEMP_DIR);

        // Try to use cached metadata if Cargo.lock hasn't changed
        if let Some(cache) = MetadataCache::load(&temp_dir)
            && cache.is_valid(workspace_root)
        {
            return Self {
                es_fluent_dep: cache.es_fluent_dep,
                es_fluent_cli_helpers_dep: cache.es_fluent_cli_helpers_dep,
                target_dir: target_dir_from_env.unwrap_or(cache.target_dir),
            };
        }

        // Cache miss or invalid, run cargo metadata
        // Use no_deps() to skip full dependency resolution - we only need workspace packages
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

        // Save to cache for next time
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

    /// Fallback: find es-fluent from the CLI's own workspace (compile-time location)
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

    /// Fallback: find es-fluent-cli-helpers from the CLI's own workspace (compile-time location)
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
}

/// Write the Cargo.toml for the runner crate.
fn write_cargo_toml(temp_dir: &Path, cargo_toml_content: &str) -> Result<()> {
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml_content)
        .context("Failed to write .es-fluent/Cargo.toml")
}

/// Write the .cargo/config.toml for the runner crate.
fn write_cargo_config(temp_dir: &Path, config_content: &str) -> Result<()> {
    let cargo_dir = temp_dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).context("Failed to create .es-fluent/.cargo directory")?;
    fs::write(cargo_dir.join("config.toml"), config_content)
        .context("Failed to write .es-fluent/.cargo/config.toml")
}

/// Run `cargo run` on the runner crate.
///
/// Returns the command stdout if cargo succeeds (captured to support diffs), or an error if it fails.
pub fn run_cargo(temp_dir: &Path, bin_name: Option<&str>, args: &[String]) -> Result<String> {
    let manifest_path = temp_dir.join("Cargo.toml");

    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    if let Some(bin) = bin_name {
        cmd.arg("--bin").arg(bin);
    }
    cmd.arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .arg("--")
        .args(args)
        .current_dir(temp_dir)
        .env("RUSTFLAGS", "-A warnings");

    // Force colored output only if NO_COLOR is NOT set
    if std::env::var("NO_COLOR").is_err() {
        cmd.env("CLICOLOR_FORCE", "1");
    }

    // Capture stdout/stderr
    let output = cmd.output().context("Failed to run cargo")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Cargo run failed: {}", stderr)
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run `cargo run` on the runner crate and capture output.
///
/// Returns the command output if successful, or an error with stderr if it fails.
pub fn run_cargo_with_output(
    temp_dir: &Path,
    bin_name: Option<&str>,
    args: &[String],
) -> Result<std::process::Output> {
    let manifest_path = temp_dir.join("Cargo.toml");

    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    if let Some(bin) = bin_name {
        cmd.arg("--bin").arg(bin);
    }
    cmd.arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .arg("--") // Add -- to pass args to the binary
        .args(args)
        .current_dir(temp_dir)
        .env("RUSTFLAGS", "-A warnings");

    let output = cmd.output().context("Failed to run cargo")?;

    if output.status.success() {
        Ok(output)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Cargo run failed: {}", stderr)
    }
}

// --- Monolithic temp crate support ---

use crate::core::WorkspaceInfo;

/// Prepare the monolithic runner crate at workspace root that links ALL workspace crates.
/// This enables fast subsequent runs by caching a single binary that can access all inventory.
pub fn prepare_monolithic_runner_crate(workspace: &WorkspaceInfo) -> Result<PathBuf> {
    let temp_dir = workspace.root_dir.join(TEMP_DIR);
    let src_dir = temp_dir.join("src");

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    // Create .gitignore
    fs::write(
        temp_dir.join(".gitignore"),
        GitignoreTemplate.render().unwrap(),
    )
    .context("Failed to write .es-fluent/.gitignore")?;

    // Get es-fluent dependency info from workspace root
    let root_manifest = workspace.root_dir.join("Cargo.toml");
    let config = TempCrateConfig::from_manifest(&root_manifest);

    // Build crate dependency list
    let crate_deps: Vec<MonolithicCrateDep> = workspace
        .crates
        .iter()
        .filter(|c| c.has_lib_rs) // Only crates with lib.rs can be linked
        .map(|c| MonolithicCrateDep {
            name: &c.name,
            path: c.manifest_dir.display().to_string(),
            ident: c.name.replace('-', "_"),
            has_features: !c.fluent_features.is_empty(),
            features: &c.fluent_features,
        })
        .collect();

    // Write Cargo.toml
    let cargo_toml = MonolithicCargoTomlTemplate {
        crates: crate_deps.clone(),
        es_fluent_dep: &config.es_fluent_dep,
        es_fluent_cli_helpers_dep: &config.es_fluent_cli_helpers_dep,
    };
    write_cargo_toml(&temp_dir, &cargo_toml.render().unwrap())?;

    // Write .cargo/config.toml for target-dir
    let config_toml = ConfigTomlTemplate {
        target_dir: &config.target_dir,
    };
    write_cargo_config(&temp_dir, &config_toml.render().unwrap())?;

    // Write main.rs
    let main_rs = MonolithicMainRsTemplate { crates: crate_deps };
    fs::write(src_dir.join("main.rs"), main_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    // Copy workspace Cargo.lock to ensure identical dependency versions.
    // The runner is a separate workspace (required to avoid "not a workspace member" errors),
    // so we copy the lock file to get the same dependency resolution as the user's workspace.
    let workspace_lock = workspace.root_dir.join("Cargo.lock");
    let runner_lock = temp_dir.join("Cargo.lock");
    if workspace_lock.exists() {
        fs::copy(&workspace_lock, &runner_lock)
            .context("Failed to copy Cargo.lock to .es-fluent/")?;
    }

    Ok(temp_dir)
}

/// Get the path to the monolithic binary if it exists.
pub fn get_monolithic_binary_path(workspace: &WorkspaceInfo) -> PathBuf {
    workspace.target_dir.join("debug").join("es-fluent-runner")
}

/// Run the monolithic binary directly (fast path) or build+run (slow path).
pub fn run_monolithic(
    workspace: &WorkspaceInfo,
    command: &str,
    crate_name: &str,
    extra_args: &[String],
) -> Result<String> {
    let temp_dir = workspace.root_dir.join(TEMP_DIR);
    let binary_path = get_monolithic_binary_path(workspace);

    // If binary exists, check if it's stale
    if binary_path.exists() && !is_runner_stale(workspace, &binary_path) {
        let mut cmd = Command::new(&binary_path);
        cmd.arg(command)
                .args(extra_args) // Put extra_args (including i18n_path) first
                .arg("--crate")
                .arg(crate_name)
                .current_dir(&temp_dir);

        // Force colored output only if NO_COLOR is NOT set
        if std::env::var("NO_COLOR").is_err() {
            cmd.env("CLICOLOR_FORCE", "1");
        }

        let output = cmd.output().context("Failed to run monolithic binary")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Monolithic binary failed: {}", stderr);
        }

        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    // Otherwise, fall back to cargo run (will build)
    // Args order: command, extra_args..., --crate, crate_name
    let mut args = vec![command.to_string()];
    args.extend(extra_args.iter().cloned());
    args.push("--crate".to_string());
    args.push(crate_name.to_string());
    let result = run_cargo(&temp_dir, Some("es-fluent-runner"), &args)?;

    // After successful cargo run, write runner cache with current per-crate hashes
    {
        use super::cache::{RunnerCache, compute_content_hash};

        let binary_path = get_monolithic_binary_path(workspace);
        if let Ok(meta) = fs::metadata(&binary_path)
            && let Ok(mtime) = meta.modified()
        {
            let runner_mtime_secs = mtime
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            // Compute per-crate content hashes (including i18n.toml)
            let mut crate_hashes = indexmap::IndexMap::new();
            for krate in &workspace.crates {
                if krate.src_dir.exists() {
                    let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
                    crate_hashes.insert(krate.name.clone(), hash);
                }
            }

            let cache = RunnerCache {
                crate_hashes,
                runner_mtime: runner_mtime_secs,
                cli_version: CLI_VERSION.to_string(),
            };
            let _ = cache.save(&temp_dir);
        }
    }

    Ok(result)
}

/// Check if the runner binary is stale compared to workspace source files.
///
/// Uses per-crate blake3 content hashing to detect actual changes - saving a file
/// without modifications won't trigger a rebuild. Hashes are stored in runner_cache.json.
///
/// Also checks CLI version - if the CLI was upgraded, the runner needs to be rebuilt
/// to pick up changes in es-fluent-cli-helpers.
fn is_runner_stale(workspace: &WorkspaceInfo, runner_path: &Path) -> bool {
    use super::cache::{RunnerCache, compute_content_hash};

    let runner_mtime = match fs::metadata(runner_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return true, // Treat as stale if we can't read metadata
    };

    let runner_mtime_secs = runner_mtime
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let temp_dir = workspace.root_dir.join(TEMP_DIR);

    // Compute current content hashes for each crate (including i18n.toml)
    let mut current_hashes = indexmap::IndexMap::new();
    for krate in &workspace.crates {
        if krate.src_dir.exists() {
            let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
            current_hashes.insert(krate.name.clone(), hash);
        }
    }

    // Check runner cache
    if let Some(cache) = RunnerCache::load(&temp_dir) {
        // Check CLI version first - if upgraded, force rebuild to pick up helper changes
        if cache.cli_version != CLI_VERSION {
            return true;
        }

        if cache.runner_mtime == runner_mtime_secs {
            // Runner hasn't been rebuilt - check if any crate content changed
            for (name, current_hash) in &current_hashes {
                match cache.crate_hashes.get(name) {
                    Some(cached_hash) if cached_hash == current_hash => continue,
                    _ => return true, // Hash mismatch or new crate
                }
            }
            // Also check for removed crates
            for cached_name in cache.crate_hashes.keys() {
                if !current_hashes.contains_key(cached_name) {
                    return true;
                }
            }
            // All hashes match, runner is fresh
            return false;
        }

        // Runner was rebuilt - update cache with current hashes and version
        let new_cache = RunnerCache {
            crate_hashes: current_hashes,
            runner_mtime: runner_mtime_secs,
            cli_version: CLI_VERSION.to_string(),
        };
        let _ = new_cache.save(&temp_dir);
        return false;
    }

    // No cache exists - be conservative and trigger rebuild
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_temp_crate_config_nonexistent_manifest() {
        let config = TempCrateConfig::from_manifest(Path::new("/nonexistent/Cargo.toml"));
        // With fallback, should find local es-fluent from CLI workspace
        // If running in CI or different environment, may still be crates.io
        assert!(config.es_fluent_dep.contains("es-fluent"));
    }

    #[test]
    fn test_temp_crate_config_non_workspace_member() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manifest_path = temp_dir.path().join("Cargo.toml");

        let cargo_toml = r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[dependencies]
es-fluent = { version = "*" }
"#;
        let mut file = fs::File::create(&manifest_path).unwrap();
        file.write_all(cargo_toml.as_bytes()).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "").unwrap();

        let config = TempCrateConfig::from_manifest(&manifest_path);
        // With fallback, should find local es-fluent from CLI workspace
        assert!(config.es_fluent_dep.contains("es-fluent"));
    }
}
