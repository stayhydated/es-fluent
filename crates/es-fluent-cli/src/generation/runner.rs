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
use es_fluent_derive_core::get_es_fluent_temp_dir;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

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
        let temp_dir = get_es_fluent_temp_dir(workspace_root);

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

struct RunnerCrate<'a> {
    temp_dir: &'a Path,
}

impl RunnerCrate<'_> {
    fn new(temp_dir: &Path) -> RunnerCrate<'_> {
        RunnerCrate { temp_dir }
    }

    fn manifest_path(&self) -> PathBuf {
        self.temp_dir.join("Cargo.toml")
    }

    /// Write the Cargo.toml for the runner crate.
    fn write_cargo_toml(&self, cargo_toml_content: &str) -> Result<()> {
        fs::write(self.temp_dir.join("Cargo.toml"), cargo_toml_content)
            .context("Failed to write .es-fluent/Cargo.toml")
    }

    /// Write the .cargo/config.toml for the runner crate.
    fn write_cargo_config(&self, config_content: &str) -> Result<()> {
        let cargo_dir = self.temp_dir.join(".cargo");
        fs::create_dir_all(&cargo_dir).context("Failed to create .es-fluent/.cargo directory")?;
        fs::write(cargo_dir.join("config.toml"), config_content)
            .context("Failed to write .es-fluent/.cargo/config.toml")
    }

    /// Run `cargo run` on the runner crate.
    ///
    /// Returns the command stdout if cargo succeeds (captured to support diffs), or an error if it fails.
    fn run_cargo(&self, bin_name: Option<&str>, args: &[String]) -> Result<String> {
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        if let Some(bin) = bin_name {
            cmd.arg("--bin").arg(bin);
        }
        cmd.arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--quiet")
            .arg("--")
            .args(args)
            .current_dir(self.temp_dir)
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
    fn run_cargo_with_output(
        &self,
        bin_name: Option<&str>,
        args: &[String],
    ) -> Result<std::process::Output> {
        let mut cmd = Command::new("cargo");
        cmd.arg("run");
        if let Some(bin) = bin_name {
            cmd.arg("--bin").arg(bin);
        }
        cmd.arg("--manifest-path")
            .arg(self.manifest_path())
            .arg("--quiet")
            .arg("--") // Add -- to pass args to the binary
            .args(args)
            .current_dir(self.temp_dir)
            .env("RUSTFLAGS", "-A warnings");

        let output = cmd.output().context("Failed to run cargo")?;

        if output.status.success() {
            Ok(output)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Cargo run failed: {}", stderr)
        }
    }
}

/// Run `cargo run` on the runner crate.
///
/// Returns the command stdout if cargo succeeds (captured to support diffs), or an error if it fails.
pub fn run_cargo(temp_dir: &Path, bin_name: Option<&str>, args: &[String]) -> Result<String> {
    RunnerCrate::new(temp_dir).run_cargo(bin_name, args)
}

/// Run `cargo run` on the runner crate and capture output.
///
/// Returns the command output if successful, or an error with stderr if it fails.
pub fn run_cargo_with_output(
    temp_dir: &Path,
    bin_name: Option<&str>,
    args: &[String],
) -> Result<std::process::Output> {
    RunnerCrate::new(temp_dir).run_cargo_with_output(bin_name, args)
}

// --- Monolithic temp crate support ---

use crate::core::WorkspaceInfo;

struct MonolithicRunner<'a> {
    workspace: &'a WorkspaceInfo,
    temp_dir: PathBuf,
    binary_path: PathBuf,
}

impl<'a> MonolithicRunner<'a> {
    fn new(workspace: &'a WorkspaceInfo) -> Self {
        Self {
            workspace,
            temp_dir: get_es_fluent_temp_dir(&workspace.root_dir),
            binary_path: get_monolithic_binary_path(workspace),
        }
    }

    fn is_stale(&self) -> bool {
        use super::cache::{RunnerCache, compute_content_hash};

        let runner_mtime = match fs::metadata(&self.binary_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return true, // Treat as stale if we can't read metadata
        };

        let runner_mtime_secs = runner_mtime
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Compute current content hashes for each crate (including i18n.toml)
        let mut current_hashes = indexmap::IndexMap::new();
        for krate in &self.workspace.crates {
            if krate.src_dir.exists() {
                let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
                current_hashes.insert(krate.name.clone(), hash);
            }
        }

        // Check runner cache
        if let Some(cache) = RunnerCache::load(&self.temp_dir) {
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
            let _ = new_cache.save(&self.temp_dir);
            return false;
        }

        // No cache exists - be conservative and trigger rebuild
        true
    }
}

/// Prepare the monolithic runner crate at workspace root that links ALL workspace crates.
/// This enables fast subsequent runs by caching a single binary that can access all inventory.
pub fn prepare_monolithic_runner_crate(workspace: &WorkspaceInfo) -> Result<PathBuf> {
    let temp_dir = get_es_fluent_temp_dir(&workspace.root_dir);
    let src_dir = temp_dir.join("src");
    let runner_crate = RunnerCrate::new(&temp_dir);

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
    runner_crate.write_cargo_toml(&cargo_toml.render().unwrap())?;

    // Write .cargo/config.toml for target-dir
    let config_toml = ConfigTomlTemplate {
        target_dir: &config.target_dir,
    };
    runner_crate.write_cargo_config(&config_toml.render().unwrap())?;

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
///
/// If `force_run` is true, the staleness check is skipped and the runner is always rebuilt.
pub fn run_monolithic(
    workspace: &WorkspaceInfo,
    command: &str,
    crate_name: &str,
    extra_args: &[String],
    force_run: bool,
) -> Result<String> {
    let runner = MonolithicRunner::new(workspace);

    // If binary exists, check if it's stale (unless force_run is set)
    if !force_run && runner.binary_path.exists() && !runner.is_stale() {
        let mut cmd = Command::new(&runner.binary_path);
        cmd.arg(command)
                .args(extra_args) // Put extra_args (including i18n_path) first
                .arg("--crate")
                .arg(crate_name)
                .current_dir(&runner.temp_dir);

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
    let result = RunnerCrate::new(&runner.temp_dir).run_cargo(Some("es-fluent-runner"), &args)?;

    // After successful cargo run, write runner cache with current per-crate hashes
    {
        use super::cache::{RunnerCache, compute_content_hash};

        if let Ok(meta) = fs::metadata(&runner.binary_path)
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
            let _ = cache.save(&runner.temp_dir);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CrateInfo;
    use crate::generation::cache::{MetadataCache, RunnerCache, compute_content_hash};
    use std::io::Write;
    use std::time::SystemTime;

    #[cfg(unix)]
    fn set_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("set executable");
    }

    #[cfg(not(unix))]
    fn set_executable(_path: &Path) {}

    fn create_workspace_fixture(
        crate_name: &str,
        has_lib_rs: bool,
    ) -> (tempfile::TempDir, WorkspaceInfo) {
        let temp = tempfile::tempdir().expect("tempdir");

        std::fs::write(
            temp.path().join("Cargo.toml"),
            format!(
                r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"
"#
            ),
        )
        .expect("write Cargo.toml");

        let src_dir = temp.path().join("src");
        std::fs::create_dir_all(&src_dir).expect("create src");
        if has_lib_rs {
            std::fs::write(src_dir.join("lib.rs"), "pub struct Demo;\n").expect("write lib.rs");
        }

        let i18n_config_path = temp.path().join("i18n.toml");
        std::fs::write(
            &i18n_config_path,
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        let krate = CrateInfo {
            name: crate_name.to_string(),
            manifest_dir: temp.path().to_path_buf(),
            src_dir,
            i18n_config_path,
            ftl_output_dir: temp.path().join("i18n/en"),
            has_lib_rs,
            fluent_features: Vec::new(),
        };

        let workspace = WorkspaceInfo {
            root_dir: temp.path().to_path_buf(),
            target_dir: temp.path().join("target"),
            crates: vec![krate],
        };

        (temp, workspace)
    }

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

    #[test]
    fn temp_crate_config_uses_valid_cached_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("Cargo.toml");
        std::fs::write(
            &manifest_path,
            r#"[package]
name = "cached"
version = "0.1.0"
edition = "2024"
"#,
        )
        .expect("write Cargo.toml");
        std::fs::write(temp.path().join("Cargo.lock"), "lock").expect("write Cargo.lock");

        let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(temp.path());
        std::fs::create_dir_all(&temp_dir).expect("create .es-fluent");
        MetadataCache {
            cargo_lock_hash: MetadataCache::hash_cargo_lock(temp.path()).expect("hash lock"),
            es_fluent_dep: "es-fluent = { path = \"/tmp/es\" }".to_string(),
            es_fluent_cli_helpers_dep: "es-fluent-cli-helpers = { path = \"/tmp/helpers\" }"
                .to_string(),
            target_dir: "/tmp/target".to_string(),
        }
        .save(&temp_dir)
        .expect("save metadata cache");

        let config = TempCrateConfig::from_manifest(&manifest_path);
        assert_eq!(config.es_fluent_dep, "es-fluent = { path = \"/tmp/es\" }");
        assert_eq!(
            config.es_fluent_cli_helpers_dep,
            "es-fluent-cli-helpers = { path = \"/tmp/helpers\" }"
        );
        assert_eq!(config.target_dir, "/tmp/target");
    }

    #[test]
    fn temp_crate_config_writes_metadata_cache_when_lock_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manifest_path = temp.path().join("Cargo.toml");
        std::fs::write(
            &manifest_path,
            r#"[package]
name = "cache-write"
version = "0.1.0"
edition = "2024"
"#,
        )
        .expect("write Cargo.toml");
        std::fs::write(temp.path().join("Cargo.lock"), "lock-content").expect("write lock");

        let _ = TempCrateConfig::from_manifest(&manifest_path);
        let temp_dir = es_fluent_derive_core::get_es_fluent_temp_dir(temp.path());
        let cache = MetadataCache::load(&temp_dir);
        assert!(cache.is_some(), "metadata cache should be written");
    }

    #[test]
    fn runner_crate_writes_manifest_and_config_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let runner = RunnerCrate::new(temp.path());

        let manifest = runner.manifest_path();
        assert_eq!(manifest, temp.path().join("Cargo.toml"));

        runner
            .write_cargo_toml("[package]\nname = \"runner\"\nversion = \"0.1.0\"\n")
            .expect("write Cargo.toml");
        runner
            .write_cargo_config("[build]\ntarget-dir = \"../target\"\n")
            .expect("write config.toml");

        assert!(temp.path().join("Cargo.toml").exists());
        assert!(temp.path().join(".cargo/config.toml").exists());
    }

    #[test]
    fn prepare_monolithic_runner_crate_writes_expected_files() {
        let (_temp, workspace) = create_workspace_fixture("test-runner", true);

        let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
        assert!(runner_dir.join("Cargo.toml").exists());
        assert!(runner_dir.join("src/main.rs").exists());
        assert!(runner_dir.join(".cargo/config.toml").exists());
        assert!(runner_dir.join(".gitignore").exists());
    }

    #[test]
    fn monolithic_runner_staleness_detects_hash_changes() {
        let (_temp, workspace) = create_workspace_fixture("stale-check", true);
        let runner = MonolithicRunner::new(&workspace);
        std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
            .expect("create binary dir");
        std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

        std::fs::write(&runner.binary_path, "#!/bin/sh\necho monolithic-runner\n")
            .expect("write fake runner");
        set_executable(&runner.binary_path);

        let mtime = std::fs::metadata(&runner.binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();

        let krate = &workspace.crates[0];
        let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: CLI_VERSION.to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save cache");

        assert!(!runner.is_stale(), "cache should mark runner as fresh");

        std::fs::write(krate.src_dir.join("lib.rs"), "pub struct Changed;\n").expect("rewrite src");
        assert!(runner.is_stale(), "content change should mark runner stale");
    }

    #[test]
    fn run_monolithic_uses_fast_path_binary_when_cache_is_fresh() {
        let (_temp, workspace) = create_workspace_fixture("fast-path", true);
        let runner = MonolithicRunner::new(&workspace);
        std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
            .expect("create binary dir");
        std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

        std::fs::write(&runner.binary_path, "#!/bin/sh\necho \"$@\"\n").expect("write fake runner");
        set_executable(&runner.binary_path);

        let mtime = std::fs::metadata(&runner.binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();

        let krate = &workspace.crates[0];
        let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: CLI_VERSION.to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save cache");

        let output = run_monolithic(
            &workspace,
            "generate",
            &krate.name,
            &["--dry-run".to_string()],
            false,
        )
        .expect("run monolithic");

        assert!(
            output.contains("generate --dry-run --crate fast-path"),
            "unexpected fast-path output: {output}"
        );
    }

    #[test]
    fn run_monolithic_fast_path_reports_binary_failure() {
        let (_temp, workspace) = create_workspace_fixture("fast-fail", true);
        let runner = MonolithicRunner::new(&workspace);
        std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
            .expect("create binary dir");
        std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

        std::fs::write(&runner.binary_path, "#!/bin/sh\necho boom 1>&2\nexit 1\n")
            .expect("write failing runner");
        set_executable(&runner.binary_path);

        let mtime = std::fs::metadata(&runner.binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();

        let krate = &workspace.crates[0];
        let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: CLI_VERSION.to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save cache");

        let err = run_monolithic(&workspace, "generate", &krate.name, &[], false)
            .err()
            .expect("expected fast-path failure");
        let msg = err.to_string();
        assert!(
            msg.contains("Monolithic binary failed")
                || msg.contains("Failed to run monolithic binary"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn run_cargo_helpers_execute_simple_temp_crate() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(temp.path().join("src")).expect("create src");
        std::fs::write(
            temp.path().join("Cargo.toml"),
            r#"[package]
name = "runner-test"
version = "0.1.0"
edition = "2024"
"#,
        )
        .expect("write Cargo.toml");
        std::fs::write(
            temp.path().join("src/main.rs"),
            r#"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join(" "));
}
"#,
        )
        .expect("write main.rs");

        let output = run_cargo(temp.path(), None, &["hello".to_string()]).expect("run cargo");
        assert!(output.contains("hello"));

        let output = run_cargo_with_output(temp.path(), None, &["world".to_string()])
            .expect("run cargo with output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("world"));

        let output = run_cargo_with_output(temp.path(), Some("runner-test"), &["bin".to_string()])
            .expect("run named bin");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("bin"));

        let err = run_cargo(temp.path(), Some("missing-bin"), &[])
            .err()
            .expect("missing bin should fail");
        assert!(err.to_string().contains("Cargo run failed"));

        let err = run_cargo_with_output(temp.path(), Some("missing-bin"), &[])
            .err()
            .expect("missing bin should fail");
        assert!(err.to_string().contains("Cargo run failed"));
    }

    #[test]
    fn create_workspace_fixture_without_lib_skips_lib_file_creation() {
        let (_temp, workspace) = create_workspace_fixture("no-lib-fixture", false);
        assert!(
            !workspace.crates[0].src_dir.join("lib.rs").exists(),
            "lib.rs should not be created when has_lib_rs is false"
        );
    }

    #[test]
    fn monolithic_runner_staleness_handles_missing_cache_and_metadata_variants() {
        let (_temp, workspace) = create_workspace_fixture("stale-variants", true);
        let runner = MonolithicRunner::new(&workspace);

        // No binary metadata available -> stale.
        assert!(runner.is_stale());

        std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
            .expect("create binary dir");
        std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");
        std::fs::write(&runner.binary_path, "#!/bin/sh\necho ok\n").expect("write fake runner");
        set_executable(&runner.binary_path);

        let mtime = std::fs::metadata(&runner.binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();

        let krate = &workspace.crates[0];
        let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        RunnerCache {
            crate_hashes: crate_hashes.clone(),
            runner_mtime: mtime,
            cli_version: "0.0.0".to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save old-version cache");
        assert!(runner.is_stale(), "version mismatch should be stale");

        crate_hashes.insert("removed-crate".to_string(), "abc".to_string());
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: CLI_VERSION.to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save removed-crate cache");
        assert!(runner.is_stale(), "removed crate should be stale");
    }

    #[test]
    fn monolithic_runner_staleness_updates_cache_when_mtime_changes() {
        let (_temp, workspace) = create_workspace_fixture("mtime-refresh", true);
        let runner = MonolithicRunner::new(&workspace);
        std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
            .expect("create binary dir");
        std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");
        std::fs::write(&runner.binary_path, "#!/bin/sh\necho ok\n").expect("write fake runner");
        set_executable(&runner.binary_path);

        let current_mtime = std::fs::metadata(&runner.binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();
        let krate = &workspace.crates[0];
        let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        RunnerCache {
            crate_hashes,
            runner_mtime: current_mtime.saturating_sub(1),
            cli_version: CLI_VERSION.to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save stale-mtime cache");

        assert!(
            !runner.is_stale(),
            "mtime mismatch should refresh cache and stay fresh"
        );
        let updated = RunnerCache::load(&runner.temp_dir).expect("load updated cache");
        assert_eq!(updated.runner_mtime, current_mtime);
    }

    #[test]
    fn prepare_monolithic_runner_crate_copies_workspace_lock_file() {
        let (_temp, workspace) = create_workspace_fixture("lock-copy", true);
        std::fs::write(workspace.root_dir.join("Cargo.lock"), "workspace-lock")
            .expect("write workspace lock");

        let runner_dir = prepare_monolithic_runner_crate(&workspace).expect("prepare runner");
        assert!(runner_dir.join("Cargo.lock").exists());
    }

    #[cfg(unix)]
    #[test]
    fn run_monolithic_fast_path_surfaces_execution_errors() {
        let (_temp, workspace) = create_workspace_fixture("fast-exec-error", true);
        let runner = MonolithicRunner::new(&workspace);
        std::fs::create_dir_all(runner.binary_path.parent().expect("binary parent"))
            .expect("create binary dir");
        std::fs::create_dir_all(&runner.temp_dir).expect("create temp dir");

        std::fs::write(&runner.binary_path, "not executable").expect("write non-executable file");

        let mtime = std::fs::metadata(&runner.binary_path)
            .and_then(|m| m.modified())
            .expect("runner mtime")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("mtime duration")
            .as_secs();
        let krate = &workspace.crates[0];
        let hash = compute_content_hash(&krate.src_dir, Some(&krate.i18n_config_path));
        let mut crate_hashes = indexmap::IndexMap::new();
        crate_hashes.insert(krate.name.clone(), hash);
        RunnerCache {
            crate_hashes,
            runner_mtime: mtime,
            cli_version: CLI_VERSION.to_string(),
        }
        .save(&runner.temp_dir)
        .expect("save cache");

        let err = run_monolithic(&workspace, "generate", &krate.name, &[], false)
            .err()
            .expect("expected execution failure");
        assert!(err.to_string().contains("Failed to run monolithic binary"));
    }

    #[test]
    fn run_monolithic_force_run_uses_slow_path_and_writes_runner_cache() {
        let (_temp, workspace) = create_workspace_fixture("slow-path", true);
        let runner_dir = es_fluent_derive_core::get_es_fluent_temp_dir(&workspace.root_dir);
        std::fs::create_dir_all(runner_dir.join("src")).expect("create runner src");
        std::fs::write(
            runner_dir.join("Cargo.toml"),
            r#"[package]
name = "dummy-runner"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "es-fluent-runner"
path = "src/main.rs"
"#,
        )
        .expect("write runner Cargo.toml");
        std::fs::write(
            runner_dir.join("src/main.rs"),
            r#"fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join(" "));
}
"#,
        )
        .expect("write runner main.rs");

        let binary_path = workspace.target_dir.join("debug/es-fluent-runner");
        std::fs::create_dir_all(binary_path.parent().unwrap()).expect("create target/debug");
        std::fs::write(&binary_path, "#!/bin/sh\necho cache-metadata\n").expect("write binary");
        set_executable(&binary_path);

        let output = run_monolithic(
            &workspace,
            "generate",
            &workspace.crates[0].name,
            &["--dry-run".to_string()],
            true,
        )
        .expect("slow path run should succeed");
        assert!(
            output.contains("generate --dry-run --crate slow-path"),
            "unexpected slow-path output: {output}"
        );

        let cache = RunnerCache::load(&runner_dir).expect("runner cache should be written");
        assert!(cache.crate_hashes.contains_key("slow-path"));
    }
}
