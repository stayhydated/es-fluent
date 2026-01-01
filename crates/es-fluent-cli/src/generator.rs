use crate::mode::{FluentParseMode, FluentParseModeExt as _};
use crate::templates::{CargoTomlTemplate, GitignoreTemplate, MainRsTemplate};
use crate::types::CrateInfo;
use anyhow::{Context as _, Result, bail};
use askama::Template as _;
use std::fs;
use std::path::Path;
use std::process::Command;

const TEMP_DIR: &str = ".es-fluent";
const TEMP_CRATE_NAME: &str = "es-fluent-gen";

/// Generates FTL files for a crate using the CrateInfo struct.
pub fn generate_for_crate(krate: &CrateInfo, mode: &FluentParseMode) -> Result<()> {
    if !krate.has_lib_rs {
        bail!(
            "Crate '{}' has no lib.rs - inventory requires a library target for linking",
            krate.name
        );
    }

    let temp_dir = create_temp_crate(krate, mode)?;
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
        let es_fluent_workspace_member = meta
            .packages
            .iter()
            .find(|p| p.name.as_str() == "es-fluent" && meta.workspace_members.contains(&p.id));

        es_fluent_workspace_member
            .map(|es_fluent_pkg| {
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
fn create_temp_crate(krate: &CrateInfo, mode: &FluentParseMode) -> Result<std::path::PathBuf> {
    let temp_dir = krate.manifest_dir.join(TEMP_DIR);
    let src_dir = temp_dir.join("src");

    fs::create_dir_all(&src_dir).context("Failed to create .es-fluent directory")?;

    // Create .gitignore to exclude the entire directory
    fs::write(
        temp_dir.join(".gitignore"),
        GitignoreTemplate.render().unwrap(),
    )
    .context("Failed to write .es-fluent/.gitignore")?;

    let crate_ident = krate.name.replace('-', "_");

    let manifest_path = krate.manifest_dir.join("Cargo.toml");
    let es_fluent_dep = get_es_fluent_dep(&manifest_path);

    let cargo_toml = CargoTomlTemplate {
        crate_name: TEMP_CRATE_NAME,
        parent_crate_name: &krate.name,
        es_fluent_dep: &es_fluent_dep,
        has_fluent_features: !krate.fluent_features.is_empty(),
        fluent_features: &krate.fluent_features,
    };
    fs::write(temp_dir.join("Cargo.toml"), cargo_toml.render().unwrap())
        .context("Failed to write .es-fluent/Cargo.toml")?;

    let i18n_toml_path_str = krate.i18n_config_path.display().to_string();
    let main_rs = MainRsTemplate {
        crate_ident: &crate_ident,
        i18n_toml_path: &i18n_toml_path_str,
        parse_mode: mode.as_code(),
        crate_name: &krate.name,
    };
    fs::write(src_dir.join("main.rs"), main_rs.render().unwrap())
        .context("Failed to write .es-fluent/src/main.rs")?;

    Ok(temp_dir)
}

fn run_cargo_bin(temp_dir: &Path) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CRATES_IO_DEP: &str = r#"es-fluent = { version = "*", features = ["generate"] }"#;

    #[test]
    fn test_get_es_fluent_dep_nonexistent_manifest() {
        // When manifest doesn't exist, should fall back to crates.io dependency
        let result = get_es_fluent_dep(Path::new("/nonexistent/Cargo.toml"));
        assert_eq!(result, CRATES_IO_DEP);
    }

    #[test]
    fn test_get_es_fluent_dep_non_workspace_member() {
        // Create a temporary crate that depends on es-fluent from crates.io
        // (not as a workspace member). Should return version dependency.
        let temp_dir = tempfile::tempdir().unwrap();
        let manifest_path = temp_dir.path().join("Cargo.toml");

        // Create a minimal Cargo.toml that references es-fluent as a dependency
        // but es-fluent is NOT a workspace member of this temporary project
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

        // Create src/lib.rs so cargo metadata works
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("lib.rs"), "").unwrap();

        let result = get_es_fluent_dep(&manifest_path);

        // es-fluent is in dependencies but NOT a workspace member,
        // so it should return the crates.io version dependency
        assert_eq!(result, CRATES_IO_DEP);
    }
}
