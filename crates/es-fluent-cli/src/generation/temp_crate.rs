//! Shared functionality for creating and running temporary crates.
//!
//! Both the generator and validator commands create temporary crates in `.es-fluent/`
//! to leverage Rust's inventory mechanism. This module consolidates that shared logic.

use crate::core::CrateInfo;
use crate::generation::GitignoreTemplate;
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The directory name for temporary crates.
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
        let metadata = cargo_metadata::MetadataCommand::new()
            .manifest_path(manifest_path)
            .exec()
            .ok();

        let (es_fluent_dep, es_fluent_cli_helpers_dep, target_dir) = match metadata {
            Some(meta) => {
                let es_fluent = Self::find_local_dep(&meta, "es-fluent")
                    .unwrap_or_else(|| r#"es-fluent = { version = "*" }"#.to_string());
                let helpers = Self::find_local_dep(&meta, "es-fluent-cli-helpers")
                    .unwrap_or_else(|| r#"es-fluent-cli-helpers = { version = "*" }"#.to_string());
                let target = target_dir_from_env
                    .unwrap_or_else(|| meta.target_directory.to_string());
                (es_fluent, helpers, target)
            }
            None => (
                r#"es-fluent = { version = "*" }"#.to_string(),
                r#"es-fluent-cli-helpers = { version = "*" }"#.to_string(),
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
}

/// Create the base temporary crate directory structure.
///
/// This creates:
/// - `.es-fluent/` directory
/// - `.es-fluent/src/` directory
/// - `.es-fluent/.gitignore`
///
/// Returns the path to the temp directory.
pub fn create_temp_dir(krate: &CrateInfo) -> Result<PathBuf> {
    let temp_dir = krate.manifest_dir.join(TEMP_DIR);
    let src_dir = temp_dir.join("src");

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    // Create .gitignore to exclude the entire directory
    fs::write(
        temp_dir.join(".gitignore"),
        GitignoreTemplate.render().unwrap(),
    )
    .context("Failed to write .es-fluent/.gitignore")?;

    Ok(temp_dir)
}

use crate::generation::{CargoTomlTemplate, CheckRsTemplate, GenerateRsTemplate};

/// Prepare the temporary crate with both generate and check binaries.
pub fn prepare_temp_crate(krate: &CrateInfo) -> Result<PathBuf> {
    let temp_dir = create_temp_dir(krate)?;

    let crate_ident = krate.name.replace('-', "_");
    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    
    // Get all config from single cargo_metadata call
    let config = TempCrateConfig::from_manifest(&manifest_path);

    let cargo_toml = CargoTomlTemplate {
        crate_name: "es-fluent-temp", // Use a generic name
        parent_crate_name: &krate.name,
        es_fluent_dep: &config.es_fluent_dep,
        es_fluent_cli_helpers_dep: &config.es_fluent_cli_helpers_dep,
        has_fluent_features: !krate.fluent_features.is_empty(),
        fluent_features: &krate.fluent_features,
        target_dir: &config.target_dir,
    };
    write_cargo_toml(&temp_dir, &cargo_toml.render().unwrap())?;

    // Write generate binary
    let i18n_toml_path_str = krate.i18n_config_path.display().to_string();
    let generate_rs = GenerateRsTemplate {
        crate_ident: &crate_ident,
        i18n_toml_path: &i18n_toml_path_str,
        crate_name: &krate.name,
    };
    write_bin_rs(&temp_dir, "generate.rs", &generate_rs.render().unwrap())?;

    // Write check binary
    let check_rs = CheckRsTemplate {
        crate_ident: &crate_ident,
        crate_name: &krate.name,
    };
    write_bin_rs(&temp_dir, "check.rs", &check_rs.render().unwrap())?;

    Ok(temp_dir)
}

/// Write a binary source file to the temporary crate.
fn write_bin_rs(temp_dir: &Path, filename: &str, content: &str) -> Result<()> {
    fs::write(temp_dir.join("src").join(filename), content)
        .with_context(|| format!("Failed to write .es-fluent/src/{}", filename))
}

/// Write the Cargo.toml for a temporary crate.
pub fn write_cargo_toml(temp_dir: &Path, cargo_toml_content: &str) -> Result<()> {
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml_content)
        .context("Failed to write .es-fluent/Cargo.toml")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CRATES_IO_ES_FLUENT: &str = r#"es-fluent = { version = "*" }"#;

    #[test]
    fn test_temp_crate_config_nonexistent_manifest() {
        let config = TempCrateConfig::from_manifest(Path::new("/nonexistent/Cargo.toml"));
        assert_eq!(config.es_fluent_dep, CRATES_IO_ES_FLUENT);
        assert_eq!(config.target_dir, "../target");
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
        assert_eq!(config.es_fluent_dep, CRATES_IO_ES_FLUENT);
    }
}
