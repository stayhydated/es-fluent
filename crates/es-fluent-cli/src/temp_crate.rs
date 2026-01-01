//! Shared functionality for creating and running temporary crates.
//!
//! Both the generator and validator commands create temporary crates in `.es-fluent/`
//! to leverage Rust's inventory mechanism. This module consolidates that shared logic.

use crate::templates::GitignoreTemplate;
use crate::types::CrateInfo;
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The directory name for temporary crates.
pub const TEMP_DIR: &str = ".es-fluent";

/// Get the es-fluent dependency string, preferring local path if in workspace.
pub fn get_es_fluent_dep(manifest_path: &Path, feature: &str) -> String {
    let crates_io_dep = format!(
        r#"es-fluent = {{ version = "*", features = ["{}"] }}"#,
        feature
    );

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .ok();

    if let Some(ref meta) = metadata {
        let es_fluent_workspace_member = meta
            .packages
            .iter()
            .find(|p| p.name.as_str() == "es-fluent" && meta.workspace_members.contains(&p.id));

        es_fluent_workspace_member
            .map(|es_fluent_pkg| {
                let es_fluent_path = es_fluent_pkg.manifest_path.parent().unwrap();
                format!(
                    r#"es-fluent = {{ path = "{}", features = ["{}"] }}"#,
                    es_fluent_path, feature
                )
            })
            .unwrap_or(crates_io_dep)
    } else {
        crates_io_dep
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

/// Write the Cargo.toml for a temporary crate.
pub fn write_cargo_toml(temp_dir: &Path, cargo_toml_content: &str) -> Result<()> {
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml_content)
        .context("Failed to write .es-fluent/Cargo.toml")
}

/// Write the main.rs for a temporary crate.
pub fn write_main_rs(temp_dir: &Path, main_rs_content: &str) -> Result<()> {
    fs::write(temp_dir.join("src").join("main.rs"), main_rs_content)
        .context("Failed to write .es-fluent/src/main.rs")
}

/// Run `cargo run` on a temporary crate.
///
/// Returns Ok(()) if cargo succeeds, or an error if it fails.
pub fn run_cargo(temp_dir: &Path) -> Result<()> {
    let manifest_path = temp_dir.join("Cargo.toml");

    let status = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .env("RUSTFLAGS", "-A warnings")
        .status()
        .context("Failed to run cargo")?;

    if status.success() {
        Ok(())
    } else {
        bail!("Cargo build failed")
    }
}

/// Run `cargo run` on a temporary crate and capture output.
///
/// Returns the command output if successful, or an error with stderr if it fails.
pub fn run_cargo_with_output(temp_dir: &Path) -> Result<std::process::Output> {
    let manifest_path = temp_dir.join("Cargo.toml");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .current_dir(temp_dir)
        .env("RUSTFLAGS", "-A warnings")
        .output()
        .context("Failed to run cargo")?;

    if output.status.success() {
        Ok(output)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Cargo build failed: {}", stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CRATES_IO_DEP_GENERATE: &str =
        r#"es-fluent = { version = "*", features = ["generate"] }"#;
    const CRATES_IO_DEP_CLI: &str = r#"es-fluent = { version = "*", features = ["cli"] }"#;

    #[test]
    fn test_get_es_fluent_dep_nonexistent_manifest() {
        let result = get_es_fluent_dep(Path::new("/nonexistent/Cargo.toml"), "generate");
        assert_eq!(result, CRATES_IO_DEP_GENERATE);
    }

    #[test]
    fn test_get_es_fluent_dep_cli_feature() {
        let result = get_es_fluent_dep(Path::new("/nonexistent/Cargo.toml"), "cli");
        assert_eq!(result, CRATES_IO_DEP_CLI);
    }

    #[test]
    fn test_get_es_fluent_dep_non_workspace_member() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manifest_path = temp_dir.path().join("Cargo.toml");

        let cargo_toml = r#"
[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[dependencies]
es-fluent = { version = "*", features = ["generate"] }
"#;
        let mut file = fs::File::create(&manifest_path).unwrap();
        file.write_all(cargo_toml.as_bytes()).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "").unwrap();

        let result = get_es_fluent_dep(&manifest_path, "generate");
        assert_eq!(result, CRATES_IO_DEP_GENERATE);
    }
}
