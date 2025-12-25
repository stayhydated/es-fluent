//! Generator module - handles injecting temporary crate and running cargo

use crate::templates::{CargoTomlTemplate, MainRsTemplate};
use anyhow::{Context, Result, bail};
use askama::Template;
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const TEMP_DIR: &str = ".es-fluent";
const TEMP_CRATE_NAME: &str = "es-fluent-gen";

/// Generates FTL files for the crate at the given path
pub fn generate_once(crate_path: &Path, package: Option<&str>) -> Result<()> {
    let crate_path = crate_path
        .canonicalize()
        .context("Failed to canonicalize crate path")?;

    let (crate_name, _manifest_path) = get_crate_info(&crate_path, package)?;

    println!(
        "{} {} {}",
        "[es-fluent]".cyan().bold(),
        "Generating FTL for".dimmed(),
        crate_name.green()
    );

    let temp_dir = create_temp_crate(&crate_path, &crate_name)?;

    // Run the generator (keep .es-fluent for incremental compilation)
    run_cargo_bin(&temp_dir)
}

/// Get crate name and manifest path
fn get_crate_info(
    crate_path: &Path,
    package: Option<&str>,
) -> Result<(String, std::path::PathBuf)> {
    let manifest_path = crate_path.join("Cargo.toml");

    if !manifest_path.exists() {
        bail!("No Cargo.toml found at {}", crate_path.display());
    }

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&manifest_path)
        .exec()
        .context("Failed to get cargo metadata")?;

    let crate_name = if let Some(pkg_name) = package {
        // Find the specified package
        metadata
            .packages
            .iter()
            .find(|p| p.name.as_str() == pkg_name)
            .map(|p| p.name.to_string())
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not found in workspace", pkg_name))?
    } else {
        // Find package by manifest path
        metadata
            .packages
            .iter()
            .find(|p| p.manifest_path == manifest_path)
            .map(|p| p.name.to_string())
            .ok_or_else(|| anyhow::anyhow!("Could not determine crate name"))?
    };

    Ok((crate_name, manifest_path))
}

/// Creates a temporary crate in .es-fluent/ that generates FTL
fn create_temp_crate(crate_path: &Path, crate_name: &str) -> Result<std::path::PathBuf> {
    let temp_dir = crate_path.join(TEMP_DIR);
    let src_dir = temp_dir.join("src");

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    // Convert crate name to valid Rust identifier (replace - with _)
    let crate_ident = crate_name.replace('-', "_");

    // Check if we're in the es-fluent workspace (local development)
    // by looking for es-fluent crate in workspace packages
    let manifest_path = crate_path.join("Cargo.toml");
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&manifest_path)
        .exec()
        .ok();

    let es_fluent_dep = if let Some(ref meta) = metadata {
        // Check if es-fluent is a local workspace member
        if let Some(es_fluent_pkg) = meta
            .packages
            .iter()
            .find(|p| p.name.as_str() == "es-fluent")
        {
            // Use path dependency to local es-fluent
            let es_fluent_path = es_fluent_pkg.manifest_path.parent().unwrap();
            format!(
                r#"es-fluent = {{ path = "{}", features = ["generate"] }}"#,
                es_fluent_path
            )
        } else {
            // Use version from crates.io
            r#"es-fluent = { version = "*", features = ["generate"] }"#.to_string()
        }
    } else {
        // Fallback to crates.io
        r#"es-fluent = { version = "*", features = ["generate"] }"#.to_string()
    };

    // Create Cargo.toml for the temp crate (with empty [workspace] to be standalone)
    let cargo_toml = CargoTomlTemplate {
        crate_name: TEMP_CRATE_NAME,
        parent_crate_name: crate_name,
        es_fluent_dep: &es_fluent_dep,
    };
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml.render().unwrap())
        .context("Failed to write .es-fluent/Cargo.toml")?;

    // Get the absolute path to the parent crate's i18n.toml
    let i18n_toml_path = crate_path.join("i18n.toml");
    let i18n_toml_path_str = i18n_toml_path.display().to_string();

    // Create main.rs with explicit config path to parent crate
    let main_rs = MainRsTemplate {
        crate_ident: &crate_ident,
        i18n_toml_path: &i18n_toml_path_str,
    };
    fs::write(src_dir.join("main.rs"), main_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    Ok(temp_dir)
}

/// Runs the temp crate to generate FTL files
fn run_cargo_bin(temp_dir: &Path) -> Result<()> {
    let start = Instant::now();

    let manifest_path = temp_dir.join("Cargo.toml");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .output()
        .context("Failed to run cargo")?;

    let elapsed = start.elapsed();

    if output.status.success() {
        println!(
            "{} {} {}",
            "[es-fluent]".cyan().bold(),
            "Generated in".dimmed(),
            humantime::format_duration(elapsed).to_string().green()
        );
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{} {}", "[es-fluent]".red().bold(), "Build failed:".red());
        eprintln!("{}", stderr);
        bail!("Cargo build failed")
    }
}
