//! Shared functionality for creating and running temporary crates.
//!
//! The CLI uses a monolithic temporary crate at workspace root that links ALL workspace
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
    /// Create config by querying cargo metadata once.
    pub fn from_manifest(manifest_path: &Path) -> Self {
        // Check CARGO_TARGET_DIR env first (doesn't need metadata)
        let target_dir_from_env = std::env::var("CARGO_TARGET_DIR").ok();

        // Try cargo metadata once for everything
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
                    .unwrap_or_else(|| r#"es-fluent = { version = "*" }"#.to_string());
                let helpers = Self::find_local_dep(meta, "es-fluent-cli-helpers")
                    .or_else(Self::find_cli_workspace_dep_helpers)
                    .unwrap_or_else(|| r#"es-fluent-cli-helpers = { version = "*" }"#.to_string());
                let target =
                    target_dir_from_env.unwrap_or_else(|| meta.target_directory.to_string());
                (es_fluent, helpers, target)
            },
            None => (
                Self::find_cli_workspace_dep_es_fluent()
                    .unwrap_or_else(|| r#"es-fluent = { version = "*" }"#.to_string()),
                Self::find_cli_workspace_dep_helpers()
                    .unwrap_or_else(|| r#"es-fluent-cli-helpers = { version = "*" }"#.to_string()),
                target_dir_from_env.unwrap_or_else(|| "../target".to_string()),
            ),
        };

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

/// Write the Cargo.toml for a temporary crate.
fn write_cargo_toml(temp_dir: &Path, cargo_toml_content: &str) -> Result<()> {
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml_content)
        .context("Failed to write .es-fluent/Cargo.toml")
}

/// Write the .cargo/config.toml for a temporary crate.
fn write_cargo_config(temp_dir: &Path, config_content: &str) -> Result<()> {
    let cargo_dir = temp_dir.join(".cargo");
    fs::create_dir_all(&cargo_dir).context("Failed to create .es-fluent/.cargo directory")?;
    fs::write(cargo_dir.join("config.toml"), config_content)
        .context("Failed to write .es-fluent/.cargo/config.toml")
}

/// Run `cargo run` on a temporary crate.
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

/// Run `cargo run` on a temporary crate and capture output.
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

/// Prepare a monolithic temporary crate at workspace root that links ALL workspace crates.
/// This enables fast subsequent runs by caching a single binary that can access all inventory.
pub fn prepare_monolithic_temp_crate(workspace: &WorkspaceInfo) -> Result<PathBuf> {
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
    run_cargo(&temp_dir, Some("es-fluent-runner"), &args)
}

/// Check if the runner binary is stale compared to workspace source files.
///
/// Returns true if any source file in any workspace crate is newer than the binary.
fn is_runner_stale(workspace: &WorkspaceInfo, runner_path: &Path) -> bool {
    let runner_mtime = match fs::metadata(runner_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return true, // Treat as stale if we can't read metadata
    };

    for krate in &workspace.crates {
        if !krate.src_dir.exists() {
            continue;
        }

        let walker = walkdir::WalkDir::new(&krate.src_dir);
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if entry.path().is_file()
                && let Ok(metadata) = entry.metadata()
                && let Ok(mtime) = metadata.modified()
                && mtime > runner_mtime
            {
                return true;
            }
        }
    }

    false
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
