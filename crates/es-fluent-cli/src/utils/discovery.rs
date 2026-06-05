use crate::core::{
    CrateInfo, DiscoveredFtlOutputDir, DiscoveredI18nConfigPath, ManifestDir, SourceDir,
    WorkspaceInfo,
};
use anyhow::{Context as _, Result};
use cargo_metadata::{MetadataCommand, TargetKind};
use es_fluent_runner::PackageName;
use es_fluent_toml::ResolvedI18nLayout;
use std::path::{Path, PathBuf};

pub(crate) enum DiscoveryScope<'a> {
    #[allow(dead_code)]
    All,
    Package(&'a str),
    RequestedPaths {
        lexical: &'a Path,
        canonical: &'a Path,
    },
}

/// Discovers workspace information including root, target dir, and all crates with i18n.toml.
/// This is used by the monolithic temp crate approach for efficient inventory collection.
#[allow(dead_code)]
pub fn discover_workspace(root_dir: &Path) -> Result<WorkspaceInfo> {
    discover_workspace_scoped(root_dir, DiscoveryScope::All)
}

pub(crate) fn discover_workspace_scoped(
    root_dir: &Path,
    scope: DiscoveryScope<'_>,
) -> Result<WorkspaceInfo> {
    let root_dir = root_dir.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize root directory {}",
            root_dir.display()
        )
    })?;

    let metadata = MetadataCommand::new()
        .current_dir(&root_dir)
        .no_deps()
        .exec()
        .context("Failed to get cargo metadata")?;

    let workspace_root: PathBuf = metadata.workspace_root.clone().into();
    let target_dir: PathBuf = metadata.target_directory.clone().into();
    let path_scope = match scope {
        DiscoveryScope::RequestedPaths { lexical, canonical } => requested_path_scope(
            &[lexical, canonical],
            &workspace_root,
            &metadata.workspace_packages(),
        ),
        DiscoveryScope::All | DiscoveryScope::Package(_) => RequestedPathScope::All,
    };

    let mut crates = Vec::new();

    for package in metadata.workspace_packages() {
        let manifest_dir: PathBuf = package.manifest_path.parent().unwrap().into();
        let include_package = match scope {
            DiscoveryScope::All => true,
            DiscoveryScope::Package(package_filter) => package.name == package_filter,
            DiscoveryScope::RequestedPaths { .. } => match &path_scope {
                RequestedPathScope::All => true,
                RequestedPathScope::None => false,
                RequestedPathScope::ManifestDir(selected) => &manifest_dir == selected,
            },
        };
        if !include_package {
            continue;
        }

        let i18n_config_path = manifest_dir.join("i18n.toml");
        if !i18n_config_path.exists() {
            continue;
        }

        let layout = ResolvedI18nLayout::from_config_path(&i18n_config_path).map_err(|error| {
            anyhow::anyhow!(
                "Failed to read {}: {error}",
                workspace_relative_path(&i18n_config_path, &workspace_root)
            )
        })?;
        let ftl_output_dir = layout.output_dir.clone();
        let fluent_features = layout.fluent_features();

        let lib_target = package.targets.iter().find(|target| {
            target.kind.iter().any(|kind| {
                matches!(
                    kind,
                    TargetKind::Lib
                        | TargetKind::RLib
                        | TargetKind::DyLib
                        | TargetKind::CDyLib
                        | TargetKind::StaticLib
                )
            })
        });
        let src_dir = lib_target
            .and_then(|target| target.src_path.parent().map(PathBuf::from))
            .unwrap_or_else(|| manifest_dir.join("src"));
        let has_lib_rs = lib_target.is_some();

        let package_name = PackageName::try_new(package.name.to_string())
            .with_context(|| format!("invalid package name `{}`", package.name))?;

        crates.push(CrateInfo {
            name: package_name,
            manifest_dir: ManifestDir::from_discovered(manifest_dir),
            src_dir: SourceDir::from_discovered(src_dir),
            i18n_config_path: DiscoveredI18nConfigPath::from_discovered(i18n_config_path),
            ftl_output_dir: DiscoveredFtlOutputDir::from_discovered(ftl_output_dir),
            has_lib_rs,
            fluent_features,
        });
    }

    // Sort by name for consistent ordering
    crates.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));

    Ok(WorkspaceInfo {
        root_dir: workspace_root,
        target_dir,
        crates,
    })
}

pub(crate) fn discover_i18n_package_names(root_dir: &Path) -> Result<Vec<String>> {
    let root_dir = root_dir.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize root directory {}",
            root_dir.display()
        )
    })?;

    let metadata = MetadataCommand::new()
        .current_dir(&root_dir)
        .no_deps()
        .exec()
        .context("Failed to get cargo metadata")?;

    let mut package_names = metadata
        .workspace_packages()
        .iter()
        .filter(|package| {
            let manifest_dir: PathBuf = package.manifest_path.parent().unwrap().into();
            manifest_dir.join("i18n.toml").exists()
        })
        .map(|package| package.name.to_string())
        .collect::<Vec<_>>();
    package_names.sort();
    Ok(package_names)
}

enum RequestedPathScope {
    All,
    None,
    ManifestDir(PathBuf),
}

fn requested_path_scope(
    requested_paths: &[&Path],
    workspace_root: &Path,
    packages: &[&cargo_metadata::Package],
) -> RequestedPathScope {
    if requested_paths.iter().any(|requested_path| {
        let is_workspace_manifest = requested_path
            .file_name()
            .is_some_and(|name| name == "Cargo.toml")
            && requested_path.parent() == Some(workspace_root);
        *requested_path == workspace_root || is_workspace_manifest
    }) {
        return RequestedPathScope::All;
    }

    let selected_manifest_dir = packages
        .iter()
        .filter_map(|package| {
            let manifest_dir: PathBuf = package.manifest_path.parent()?.into();
            requested_paths
                .iter()
                .any(|requested_path| requested_path.starts_with(&manifest_dir))
                .then_some(manifest_dir)
        })
        .max_by_key(|path| path.components().count());

    if let Some(manifest_dir) = selected_manifest_dir {
        return RequestedPathScope::ManifestDir(manifest_dir);
    }

    if requested_paths
        .iter()
        .any(|requested_path| requested_path.starts_with(workspace_root))
    {
        return RequestedPathScope::None;
    }

    RequestedPathScope::All
}

/// Discovers all crates in a workspace (or single crate) that have i18n.toml.
/// This is a convenience wrapper around discover_workspace that returns just the crates.
#[cfg(test)]
pub fn discover_crates(root_dir: &Path) -> Result<Vec<CrateInfo>> {
    discover_workspace(root_dir).map(|ws| ws.crates)
}

fn workspace_relative_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Counts the number of FTL resources (message keys) for a specific crate.
pub fn count_ftl_resources(ftl_output_dir: &Path, crate_name: &str) -> usize {
    let Ok(files) = crate::ftl::discover_crate_ftl_files_in_locale_dir(ftl_output_dir, crate_name)
    else {
        return 0;
    };

    files
        .into_iter()
        .filter_map(|file| crate::ftl::parse_ftl_file(&file.abs_path).ok())
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

    use crate::test_fixtures::{LIB_RS, WORKSPACE_CARGO_TOML};

    fn create_workspace_without_i18n_toml() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("tempdir");
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
    fn discover_workspace_canonicalize_error_includes_requested_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing = temp.path().join("missing-workspace");

        let err = discover_workspace(&missing).expect_err("missing workspace should fail");

        let message = err.to_string();
        assert!(message.contains("Failed to canonicalize root directory"));
        assert!(
            message.contains(&missing.display().to_string()),
            "error should include the requested path, got {message}"
        );
    }

    #[test]
    fn discover_workspace_finds_i18n_enabled_crate() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        let ws = discover_workspace(temp.path()).expect("discover workspace");

        assert_eq!(ws.crates.len(), 1);
        let krate = &ws.crates[0];
        assert_eq!(krate.name, "test-app");
        assert!(krate.has_lib_rs);
        assert!(krate.i18n_config_path.ends_with("i18n.toml"));
    }

    #[test]
    fn discover_workspace_recognizes_custom_library_target_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"custom-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"lib.rs\"\n",
        )
        .expect("write Cargo.toml");
        fs::write(temp.path().join("lib.rs"), LIB_RS).expect("write custom lib");
        fs::write(
            temp.path().join("i18n.toml"),
            "fallback_language = \"en\"\nassets_dir = \"i18n\"\n",
        )
        .expect("write i18n.toml");

        let ws = discover_workspace(temp.path()).expect("discover workspace");

        assert_eq!(ws.crates.len(), 1);
        assert!(ws.crates[0].has_lib_rs);
        assert_eq!(ws.crates[0].src_dir.as_path(), temp.path());
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
        let temp = tempfile::tempdir().expect("tempdir");
        let err = discover_workspace(temp.path()).expect_err("expected cargo metadata failure");
        assert!(err.to_string().contains("cargo metadata") || err.to_string().contains("manifest"));
    }

    #[test]
    fn discover_workspace_errors_for_invalid_i18n_toml() {
        let temp = crate::test_fixtures::create_test_crate_workspace();
        fs::write(temp.path().join("i18n.toml"), "not = [valid").expect("write invalid i18n");

        let err = discover_workspace(temp.path()).expect_err("expected i18n parse failure");
        let message = err.to_string();
        assert!(message.contains("Failed to read i18n.toml"));
        assert!(
            !message.contains(temp.path().to_string_lossy().as_ref()),
            "discovery config errors should use workspace-relative paths: {message}"
        );
    }

    #[test]
    fn discover_workspace_collects_fluent_features_and_sorts_crates() {
        let temp = tempfile::tempdir().expect("tempdir");
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
                    "fallback_language = \"en\"\nassets_dir = \"i18n\"\nfluent_feature = [\"{feature}\"]\n"
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
        let temp = tempfile::tempdir().expect("tempdir");
        let locale_dir = temp.path().join("en");
        fs::create_dir_all(locale_dir.join("test-crate.ftl")).expect("create fake ftl dir");

        assert_eq!(count_ftl_resources(&locale_dir, "test-crate"), 0);
    }
}
