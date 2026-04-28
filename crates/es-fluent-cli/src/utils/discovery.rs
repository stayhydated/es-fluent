use crate::core::{CrateInfo, WorkspaceInfo};
use crate::ftl::{discover_crate_ftl_files_in_locale_dir, parse_ftl_file};
use anyhow::{Context as _, Result};
use cargo_metadata::MetadataCommand;
use es_fluent_toml::ResolvedI18nLayout;
use std::path::{Path, PathBuf};

/// Discovers workspace information including root, target dir, and all crates with i18n.toml.
/// This is used by the monolithic temp crate approach for efficient inventory collection.
pub fn discover_workspace(root_dir: &Path) -> Result<WorkspaceInfo> {
    let root_dir = root_dir
        .canonicalize()
        .context("Failed to canonicalize root directory")?;

    let metadata = MetadataCommand::new()
        .current_dir(&root_dir)
        .no_deps()
        .exec()
        .context("Failed to get cargo metadata")?;

    let workspace_root: PathBuf = metadata.workspace_root.clone().into();
    let target_dir: PathBuf = metadata.target_directory.clone().into();

    let mut crates = Vec::new();

    for package in metadata.workspace_packages() {
        let manifest_dir: PathBuf = package.manifest_path.parent().unwrap().into();

        let i18n_config_path = manifest_dir.join("i18n.toml");
        if !i18n_config_path.exists() {
            continue;
        }

        let layout = ResolvedI18nLayout::from_config_path(&i18n_config_path)
            .with_context(|| format!("Failed to read {}", i18n_config_path.display()))?;
        let ftl_output_dir = layout.output_dir.clone();
        let fluent_features = layout.fluent_features();

        let src_dir = manifest_dir.join("src");
        let has_lib_rs = src_dir.join("lib.rs").exists();

        crates.push(CrateInfo {
            name: package.name.to_string(),
            manifest_dir,
            src_dir,
            i18n_config_path,
            ftl_output_dir,
            has_lib_rs,
            fluent_features,
        });
    }

    // Sort by name for consistent ordering
    crates.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(WorkspaceInfo {
        root_dir: workspace_root,
        target_dir,
        crates,
    })
}

/// Discovers all crates in a workspace (or single crate) that have i18n.toml.
/// This is a convenience wrapper around discover_workspace that returns just the crates.
#[cfg(test)]
pub fn discover_crates(root_dir: &Path) -> Result<Vec<CrateInfo>> {
    discover_workspace(root_dir).map(|ws| ws.crates)
}

/// Counts the number of FTL resources (message keys) for a specific crate.
pub fn count_ftl_resources(ftl_output_dir: &Path, crate_name: &str) -> usize {
    let Ok(files) = discover_crate_ftl_files_in_locale_dir(ftl_output_dir, crate_name) else {
        return 0;
    };

    files
        .into_iter()
        .filter_map(|file| parse_ftl_file(&file.abs_path).ok())
        .map(|resource| {
            resource
                .body
                .iter()
                .filter(|entry| {
                    matches!(
                        entry,
                        fluent_syntax::ast::Entry::Message(_) | fluent_syntax::ast::Entry::Term(_)
                    )
                })
                .count()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    use crate::test_fixtures::{LIB_RS, WORKSPACE_CARGO_TOML, create_test_crate_workspace};

    fn create_workspace_without_i18n_toml() -> tempfile::TempDir {
        let temp = tempdir().expect("tempdir");
        fs::create_dir_all(temp.path().join("src")).expect("create src");
        fs::write(
            temp.path().join("Cargo.toml"),
            crate::test_fixtures::CARGO_TOML,
        )
        .expect("write Cargo.toml");
        fs::write(temp.path().join("src/lib.rs"), LIB_RS).expect("write lib.rs");
        temp
    }

    #[test]
    fn test_count_ftl_resources_empty() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(count_ftl_resources(temp.path(), "test-crate"), 0);
    }

    #[test]
    fn test_count_ftl_resources_nonexistent() {
        assert_eq!(
            count_ftl_resources(Path::new("/nonexistent/path"), "test-crate"),
            0
        );
    }

    #[test]
    fn discover_workspace_finds_i18n_enabled_crate() {
        let temp = create_test_crate_workspace();
        let ws = discover_workspace(temp.path()).expect("discover workspace");

        assert_eq!(ws.crates.len(), 1);
        let krate = &ws.crates[0];
        assert_eq!(krate.name, "test-app");
        assert!(krate.has_lib_rs);
        assert!(krate.i18n_config_path.ends_with("i18n.toml"));
    }

    #[test]
    fn discover_crates_ignores_crates_without_i18n_toml() {
        let temp = create_workspace_without_i18n_toml();
        let crates = discover_crates(temp.path()).expect("discover crates");
        assert!(crates.is_empty());
    }

    #[test]
    fn count_ftl_resources_counts_only_message_lines() {
        let temp = tempfile::tempdir().expect("tempdir");
        let locale_dir = temp.path().join("en");
        fs::create_dir_all(&locale_dir).expect("create locale");
        fs::write(
            locale_dir.join("test-crate.ftl"),
            "# comment\nhello = Hello\n  .attr = Attr\n\n-world = Term\nplain = Value\n",
        )
        .expect("write ftl");

        // Count logic is line-based and should count `hello`, `-world`, and `plain`.
        assert_eq!(count_ftl_resources(&locale_dir, "test-crate"), 3);
    }

    #[test]
    fn discover_workspace_errors_without_cargo_manifest() {
        let temp = tempdir().expect("tempdir");
        let err = discover_workspace(temp.path()).expect_err("expected cargo metadata failure");
        assert!(err.to_string().contains("cargo metadata") || err.to_string().contains("manifest"));
    }

    #[test]
    fn discover_workspace_errors_for_invalid_i18n_toml() {
        let temp = create_test_crate_workspace();
        fs::write(temp.path().join("i18n.toml"), "not = [valid").expect("write invalid i18n");

        let err = discover_workspace(temp.path()).expect_err("expected i18n parse failure");
        assert!(err.to_string().contains("Failed to read"));
    }

    #[test]
    fn discover_workspace_collects_fluent_features_and_sorts_crates() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("Cargo.toml"), WORKSPACE_CARGO_TOML)
            .expect("write workspace Cargo.toml");

        for (name, feature) in [("zeta", "z_feature"), ("alpha", "a_feature")] {
            let crate_dir = temp.path().join(name);
            fs::create_dir_all(crate_dir.join("src")).expect("create src");
            fs::write(
                crate_dir.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            )
            .expect("write crate Cargo.toml");
            fs::write(crate_dir.join("src/lib.rs"), LIB_RS).expect("write lib.rs");
            fs::write(
                crate_dir.join("i18n.toml"),
                format!(
                    "fallback_language = \"en\"\nassets_dir = \"i18n\"\nfluent_feature = \"{feature}\"\n"
                ),
            )
            .expect("write i18n.toml");
        }

        let ws = discover_workspace(temp.path()).expect("discover workspace");
        assert_eq!(ws.crates.len(), 2);
        assert_eq!(ws.crates[0].name, "alpha");
        assert_eq!(ws.crates[1].name, "zeta");
        assert_eq!(ws.crates[0].fluent_features, vec!["a_feature".to_string()]);
        assert_eq!(ws.crates[1].fluent_features, vec!["z_feature".to_string()]);
    }

    #[test]
    fn count_ftl_resources_returns_zero_when_ftl_path_is_directory() {
        let temp = tempdir().expect("tempdir");
        let locale_dir = temp.path().join("en");
        fs::create_dir_all(locale_dir.join("test-crate.ftl")).expect("create fake ftl dir");

        assert_eq!(count_ftl_resources(&locale_dir, "test-crate"), 0);
    }
}
