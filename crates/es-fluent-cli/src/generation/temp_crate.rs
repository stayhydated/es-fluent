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

/// Generic workspace dependency resolver.
///
/// Checks if the specified crate is a workspace member and returns a path-based
/// dependency if so, otherwise returns the crates.io fallback.
fn get_workspace_dep(
    manifest_path: &Path,
    crate_name: &str,
    crates_io_dep: &str,
    local_dep_template: impl Fn(&str) -> String,
) -> String {
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .ok();

    let Some(meta) = metadata else {
        return crates_io_dep.to_string();
    };

    meta.packages
        .iter()
        .find(|p| p.name.as_str() == crate_name && p.source.is_none())
        .map(|pkg| {
            let path = pkg.manifest_path.parent().unwrap();
            local_dep_template(path.as_ref())
        })
        .unwrap_or_else(|| crates_io_dep.to_string())
}

/// Get the es-fluent dependency string, preferring local path if in workspace.
pub fn get_es_fluent_dep(manifest_path: &Path) -> String {
    get_workspace_dep(
        manifest_path,
        "es-fluent",
        r#"es-fluent = { version = "*" }"#,
        |path| format!(r#"es-fluent = {{ path = "{}" }}"#, path),
    )
}

/// Get the es-fluent-cli-helpers dependency string, preferring local path if in workspace.
pub fn get_es_fluent_cli_helpers_dep(manifest_path: &Path) -> String {
    get_workspace_dep(
        manifest_path,
        "es-fluent-cli-helpers",
        r#"es-fluent-cli-helpers = { version = "*" }"#,
        |path| format!(r#"es-fluent-cli-helpers = {{ path = "{}" }}"#, path),
    )
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
    let es_fluent_dep = get_es_fluent_dep(&manifest_path);
    let es_fluent_cli_helpers_dep = get_es_fluent_cli_helpers_dep(&manifest_path);

    let cargo_toml = CargoTomlTemplate {
        crate_name: "es-fluent-temp", // Use a generic name
        parent_crate_name: &krate.name,
        es_fluent_dep: &es_fluent_dep,
        es_fluent_cli_helpers_dep: &es_fluent_cli_helpers_dep,
        has_fluent_features: !krate.fluent_features.is_empty(),
        fluent_features: &krate.fluent_features,
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

    const CRATES_IO_DEP: &str = r#"es-fluent = { version = "*" }"#;

    #[test]
    fn test_get_es_fluent_dep_nonexistent_manifest() {
        let result = get_es_fluent_dep(Path::new("/nonexistent/Cargo.toml"));
        assert_eq!(result, CRATES_IO_DEP);
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
es-fluent = { version = "*" }
"#;
        let mut file = fs::File::create(&manifest_path).unwrap();
        file.write_all(cargo_toml.as_bytes()).unwrap();

        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "").unwrap();

        let result = get_es_fluent_dep(&manifest_path);
        assert_eq!(result, CRATES_IO_DEP);
    }
}
