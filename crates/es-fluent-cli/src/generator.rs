//! Generator module - handles injecting temporary crate and running cargo

use crate::templates::{CargoTomlTemplate, MainRsTemplate};
use crate::types::CrateInfo;
use anyhow::{Context, Result, bail};
use askama::Template;
use std::fs;
use std::path::Path;
use std::process::Command;

const TEMP_DIR: &str = ".es-fluent";
const TEMP_CRATE_NAME: &str = "es-fluent-gen";

/// Generates FTL files for a crate using the CrateInfo struct.
pub fn generate_for_crate(krate: &CrateInfo) -> Result<()> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    let temp_dir = create_temp_crate(krate)?;
    run_cargo_bin(&temp_dir)
}

/// Get the es-fluent dependency string, preferring local path if in workspace.
fn get_es_fluent_dep(manifest_path: &Path) -> String {
    const CRATES_IO_DEP: &str = r#"es-fluent = { version = "*", features = ["generate"] }"#;

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()
        .ok();

    if let Some(ref meta) = metadata {
        // Check if es-fluent is a local workspace member
        meta.packages
            .iter()
            .find(|p| p.name.as_str() == "es-fluent")
            .map(|es_fluent_pkg| {
                // Use path dependency to local es-fluent
                let es_fluent_path = es_fluent_pkg.manifest_path.parent().unwrap();
                format!(
                    r#"es-fluent = {{ path = "{}", features = ["generate"] }}"#,
                    es_fluent_path
                )
            })
            .unwrap_or_else(|| CRATES_IO_DEP.to_string())
    } else {
        CRATES_IO_DEP.to_string()
    }
}

/// Creates a temporary crate in .es-fluent/ that generates FTL.
fn create_temp_crate(krate: &CrateInfo) -> Result<std::path::PathBuf> {
    let temp_dir = krate.manifest_dir.join(TEMP_DIR);
    let src_dir = temp_dir.join("src");

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    // Convert crate name to valid Rust identifier
    let crate_ident = krate.name.replace('-', "_");

    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let es_fluent_dep = get_es_fluent_dep(&manifest_path);

    // Create Cargo.toml
    let cargo_toml = CargoTomlTemplate {
        crate_name: TEMP_CRATE_NAME,
        parent_crate_name: &krate.name,
        es_fluent_dep: &es_fluent_dep,
    };
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml.render().unwrap())
        .context("Failed to write .es-fluent/Cargo.toml")?;

    // Create main.rs
    let i18n_toml_path_str = krate.i18n_config_path.display().to_string();
    let main_rs = MainRsTemplate {
        crate_ident: &crate_ident,
        i18n_toml_path: &i18n_toml_path_str,
    };
    fs::write(src_dir.join("main.rs"), main_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    Ok(temp_dir)
}

/// Runs the temp crate to generate FTL files.
fn run_cargo_bin(temp_dir: &Path) -> Result<()> {
    let manifest_path = temp_dir.join("Cargo.toml");

    let output = Command::new("cargo")
        .arg("run")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--quiet")
        .output()
        .context("Failed to run cargo")?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Filter out common cargo noise
        let error_msg: String = stderr
            .lines()
            .filter(|line| {
                !line.trim().is_empty()
                    && !line.contains("Compiling")
                    && !line.contains("Finished")
                    && !line.contains("Running")
            })
            .collect::<Vec<_>>()
            .join("\n");

        if error_msg.is_empty() {
            bail!("Cargo build failed")
        } else {
            bail!("{}", error_msg)
        }
    }
}
