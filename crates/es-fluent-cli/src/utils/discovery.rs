use crate::core::{CrateInfo, WorkspaceInfo};
use anyhow::{Context as _, Result};
use cargo_metadata::MetadataCommand;
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

        let i18n_config = es_fluent_toml::I18nConfig::read_from_path(&i18n_config_path)
            .with_context(|| format!("Failed to read {}", i18n_config_path.display()))?;

        let ftl_output_dir = manifest_dir
            .join(&i18n_config.assets_dir)
            .join(&i18n_config.fallback_language);

        let src_dir = manifest_dir.join("src");
        let has_lib_rs = src_dir.join("lib.rs").exists();

        crates.push(CrateInfo {
            name: package.name.to_string(),
            manifest_dir,
            src_dir,
            i18n_config_path,
            ftl_output_dir,
            has_lib_rs,
            fluent_features: i18n_config
                .fluent_feature
                .as_ref()
                .map(|f| f.as_vec())
                .unwrap_or_default(),
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
pub fn discover_crates(root_dir: &Path) -> Result<Vec<CrateInfo>> {
    discover_workspace(root_dir).map(|ws| ws.crates)
}

/// Counts the number of FTL resources (message keys) for a specific crate.
pub fn count_ftl_resources(ftl_output_dir: &Path, crate_name: &str) -> usize {
    let ftl_file = ftl_output_dir.join(format!("{}.ftl", crate_name));

    if !ftl_file.exists() {
        return 0;
    }

    let Ok(content) = std::fs::read_to_string(&ftl_file) else {
        return 0;
    };

    // Count lines that start with a message identifier
    // (not comments, not blank, not starting with whitespace)
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !line.starts_with(' ')
                && !line.starts_with('\t')
                && trimmed.contains('=')
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
