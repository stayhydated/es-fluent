use crate::core::{CliError, CrateInfo, WorkspaceInfo};
use crate::utils::ui;
use anyhow::Context as _;
use clap::Args;
use std::path::{Component, Path, PathBuf};

#[derive(Args, Clone, Debug)]
pub struct WorkspaceArgs {
    /// Existing path to a crate/workspace root, its Cargo.toml, or a path inside a crate (defaults to current directory).
    #[arg(short = 'P', long)]
    pub path: Option<PathBuf>,
    /// Workspace package name to process, even when --path points inside a different member.
    #[arg(short, long)]
    pub package: Option<String>,
}

/// Represents a resolved set of crates for a command to operate on.
#[derive(Clone, Debug)]
pub struct WorkspaceCrates {
    /// Workspace information (root dir, target dir, all crates).
    pub workspace_info: WorkspaceInfo,
    /// All crates discovered (after optional package filtering).
    pub crates: Vec<CrateInfo>,
    /// Crates that are eligible for operations (have a Cargo library target).
    pub valid: Vec<CrateInfo>,
    /// Crates that were skipped (missing a Cargo library target).
    pub skipped: Vec<CrateInfo>,
    /// Package filter that matched no crates, if one was supplied.
    pub(crate) package_not_found: Option<String>,
    /// All workspace packages that have an i18n.toml, without parsing each config.
    pub(crate) all_i18n_package_names: Vec<String>,
}

impl WorkspaceCrates {
    /// Discover crates for a command, applying the common filtering and partitioning logic.
    pub fn discover(args: WorkspaceArgs) -> Result<Self, CliError> {
        let WorkspaceArgs { path, package } = normalize_workspace_args(args)?;
        let path = path.unwrap_or_else(|| PathBuf::from("."));
        let package_filter = package.clone();
        let requested_path = crate::utils::paths::normalize_windows_verbatim_path(
            &path
                .canonicalize()
                .with_context(|| {
                    format!("Failed to canonicalize root directory {}", path.display())
                })
                .map_err(CliError::from)?,
        );
        let lexical_requested_path = lexical_absolute_path(&path).map_err(CliError::from)?;
        let metadata_dir =
            workspace_metadata_dir(&lexical_requested_path, requested_path.as_path());
        let discovery_scope = if let Some(package) = package.as_deref() {
            crate::utils::DiscoveryScope::Package(package)
        } else {
            crate::utils::DiscoveryScope::RequestedPaths {
                lexical: lexical_requested_path.as_path(),
                canonical: requested_path.as_path(),
            }
        };
        let all_i18n_package_names = crate::utils::discover_i18n_package_names(&metadata_dir)?;
        let workspace_info =
            crate::utils::discover_workspace_scoped(&metadata_dir, discovery_scope)?;
        let crates = if package.is_some() {
            crate::utils::filter_crates_by_package(workspace_info.crates.clone(), package.as_ref())
        } else {
            crates_for_requested_path(
                workspace_info.crates.clone(),
                &workspace_info,
                &[lexical_requested_path.as_path(), requested_path.as_path()],
            )
        };
        let package_not_found = package_filter.filter(|_| crates.is_empty());
        let (valid_refs, skipped_refs) = crate::utils::partition_by_lib_rs(&crates);
        let valid = valid_refs.into_iter().cloned().collect();
        let skipped = skipped_refs.into_iter().cloned().collect();

        Ok(Self {
            workspace_info,
            crates,
            valid,
            skipped,
            package_not_found,
            all_i18n_package_names,
        })
    }

    /// Print a standardized discovery summary, including skipped crates.
    ///
    /// Returns `false` when no crates were discovered to allow early-exit flows.
    pub fn print_discovery(&self, header: impl Fn()) -> bool {
        header();

        if self.crates.is_empty() {
            self.print_no_crates_found();
            return false;
        }

        ui::Ui::print_discovered(&self.crates);

        for krate in &self.skipped {
            ui::Ui::print_missing_lib_rs(krate.name.as_str());
        }

        true
    }

    /// Print the appropriate empty-selection message.
    pub fn print_no_crates_found(&self) {
        if let Some(package) = &self.package_not_found {
            ui::Ui::print_package_not_found(package);
        } else {
            ui::Ui::print_no_crates_found();
        }
    }

    /// Return an actionable message for an empty command selection.
    pub fn empty_selection_message(&self) -> Option<String> {
        if !self.crates.is_empty() {
            return None;
        }

        Some(if let Some(package) = &self.package_not_found {
            format!("no configured crate found matching package filter '{package}'")
        } else {
            "no crates with i18n.toml were found".to_string()
        })
    }

    /// Require a command to have at least one selected crate.
    pub fn require_non_empty_selection(&self) -> Result<(), CliError> {
        if let Some(message) = self.empty_selection_message() {
            return Err(CliError::Other(message));
        }

        Ok(())
    }

    /// Require every selected crate to have a Cargo library target.
    pub fn require_all_crates_valid(&self) -> Result<(), CliError> {
        if !self.skipped.is_empty() {
            let crate_names = self
                .skipped
                .iter()
                .map(|krate| format!("'{}'", krate.name))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(CliError::Other(format!(
                "configured crate(s) missing a Cargo library target: {crate_names}"
            )));
        }

        if self.valid.is_empty() {
            return Err(CliError::Other(
                "no discovered crates have a Cargo library target".to_string(),
            ));
        }

        Ok(())
    }
}

fn normalize_workspace_args(args: WorkspaceArgs) -> Result<WorkspaceArgs, CliError> {
    if let Some(path) = args.path.as_ref()
        && path.as_os_str().to_string_lossy().trim().is_empty()
    {
        return Err(CliError::Other(
            "workspace path must not be empty; pass a path or omit --path".to_string(),
        ));
    }

    let package = match args.package {
        Some(package) => {
            let package = package.trim();
            if package.is_empty() {
                return Err(CliError::Other(
                    "package filter must not be empty; pass a Cargo package name or omit --package"
                        .to_string(),
                ));
            }
            Some(package.to_string())
        },
        None => None,
    };

    Ok(WorkspaceArgs {
        path: args.path,
        package,
    })
}

fn lexical_absolute_path(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("Failed to read current directory")?
            .join(path)
    };
    Ok(normalize_lexical_path(&absolute))
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {},
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            },
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    crate::utils::paths::normalize_windows_verbatim_path(&normalized)
}

fn workspace_metadata_dir(
    lexical_requested_path: &Path,
    canonical_requested_path: &Path,
) -> PathBuf {
    let lexical_start = if canonical_requested_path.is_file() {
        lexical_requested_path
            .parent()
            .unwrap_or(lexical_requested_path)
    } else {
        lexical_requested_path
    };

    if let Some(manifest_ancestor) = lexical_start
        .ancestors()
        .find(|ancestor| ancestor.join("Cargo.toml").is_file())
    {
        return crate::utils::paths::normalize_windows_verbatim_path(manifest_ancestor);
    }

    if canonical_requested_path.is_file() {
        canonical_requested_path
            .parent()
            .map(crate::utils::paths::normalize_windows_verbatim_path)
            .unwrap_or_else(|| {
                crate::utils::paths::normalize_windows_verbatim_path(canonical_requested_path)
            })
    } else {
        crate::utils::paths::normalize_windows_verbatim_path(canonical_requested_path)
    }
}

fn crates_for_requested_path(
    crates: Vec<CrateInfo>,
    workspace_info: &WorkspaceInfo,
    requested_paths: &[&Path],
) -> Vec<CrateInfo> {
    let requested_paths = requested_paths
        .iter()
        .map(|path| crate::utils::paths::normalize_windows_verbatim_path(path))
        .collect::<Vec<_>>();
    let workspace_root =
        crate::utils::paths::normalize_windows_verbatim_path(&workspace_info.root_dir);

    if requested_paths.iter().any(|requested_path| {
        let is_workspace_manifest = requested_path
            .file_name()
            .is_some_and(|name| name == "Cargo.toml")
            && requested_path.parent() == Some(workspace_root.as_path());
        requested_path == &workspace_root || is_workspace_manifest
    }) {
        return crates;
    }

    if let Some(manifest_dir) = crates
        .iter()
        .filter(|krate| {
            let manifest_dir =
                crate::utils::paths::normalize_windows_verbatim_path(krate.manifest_dir.as_path());
            requested_paths
                .iter()
                .any(|requested_path| requested_path.starts_with(&manifest_dir))
        })
        .map(|krate| {
            crate::utils::paths::normalize_windows_verbatim_path(krate.manifest_dir.as_path())
        })
        .max_by_key(|path| path.components().count())
    {
        return crates
            .into_iter()
            .filter(|krate| {
                crate::utils::paths::normalize_windows_verbatim_path(krate.manifest_dir.as_path())
                    == manifest_dir
            })
            .collect();
    }

    if requested_paths
        .iter()
        .any(|requested_path| requested_path.starts_with(&workspace_root))
    {
        return Vec::new();
    }

    crates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_workspace_args_trims_package_filter() {
        let args = normalize_workspace_args(WorkspaceArgs {
            path: None,
            package: Some(" test-app ".to_string()),
        })
        .expect("package filter should normalize");

        assert_eq!(args.package.as_deref(), Some("test-app"));
    }

    #[test]
    fn normalize_workspace_args_rejects_empty_package_filter() {
        let err = normalize_workspace_args(WorkspaceArgs {
            path: None,
            package: Some(" ".to_string()),
        })
        .expect_err("empty package filter should fail");

        assert!(err.to_string().contains("package filter must not be empty"));
    }

    #[test]
    fn normalize_workspace_args_rejects_blank_path() {
        let err = normalize_workspace_args(WorkspaceArgs {
            path: Some(PathBuf::from("   ")),
            package: None,
        })
        .expect_err("blank path should fail");

        assert!(err.to_string().contains("workspace path must not be empty"));
    }
}
