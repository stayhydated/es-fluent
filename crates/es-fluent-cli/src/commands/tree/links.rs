use super::{source_map::SourcePosition, validation::validate_tree_workspace_setup};

use super::super::common::WorkspaceCrates;

use crate::core::{CliError, WorkspaceInfo};

use crate::generation::MonolithicExecutor;

use anyhow::Result;

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TreeLinkMode {
    /// Link message and variable rows to Rust source locations when available.
    #[default]
    Rust,
    /// Link message, attribute, and variable rows to FTL source locations.
    Ftl,
}

impl TreeLinkMode {
    pub(super) fn parse_arg(value: &str) -> Result<Self, CliError> {
        match value {
            "rust" => Ok(Self::Rust),
            "ftl" => Ok(Self::Ftl),
            _ => Err(CliError::Other(format!(
                "invalid link mode '{value}'; expected 'rust' or 'ftl'"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct RustEntryLink {
    pub(super) path: PathBuf,
    pub(super) position: Option<SourcePosition>,
    pub(super) variables: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct RustLinkIndex {
    pub(super) entries: HashMap<String, RustEntryLink>,
}

impl RustLinkIndex {
    pub(super) fn from_inventory(
        manifest_dir: &Path,
        inventory: es_fluent_runner::InventoryData,
    ) -> Self {
        let mut entries = HashMap::new();
        let mut ambiguous_ids = HashSet::new();
        for key in inventory.expected_keys {
            let id = key.key.id().as_str().to_string();
            if ambiguous_ids.contains(&id) {
                continue;
            }
            let Some(source_file) = key.source_file else {
                continue;
            };
            let path = absolute_source_path(manifest_dir, source_file.as_str());
            let position = key.source_line.map(|line| SourcePosition {
                line: line.get() as usize,
                column: 1,
            });
            let link = RustEntryLink {
                path,
                position,
                variables: key
                    .variables
                    .into_iter()
                    .map(|variable| variable.into_string())
                    .collect(),
            };
            if entries.insert(id.clone(), link).is_some() {
                entries.remove(&id);
                ambiguous_ids.insert(id);
            }
        }

        Self { entries }
    }

    pub(super) fn get(&self, key: &str) -> Option<&RustEntryLink> {
        self.entries.get(key)
    }
}
pub(super) fn file_url(path: &Path, position: Option<SourcePosition>) -> String {
    match position {
        Some(position) => format!(
            "file://{}:{}:{}",
            path.display(),
            position.line,
            position.column
        ),
        None => format!("file://{}", path.display()),
    }
}

pub(super) fn absolute_source_path(manifest_dir: &Path, source_file: &str) -> PathBuf {
    let source_path = Path::new(source_file);
    if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        manifest_dir.join(source_path)
    }
}

pub(super) fn collect_rust_link_indexes(
    workspace: &WorkspaceCrates,
    link_mode: TreeLinkMode,
    terminal_links: bool,
    all_locales: bool,
) -> Result<HashMap<String, RustLinkIndex>, CliError> {
    if !terminal_links || link_mode != TreeLinkMode::Rust || workspace.valid.is_empty() {
        return Ok(HashMap::new());
    }
    validate_tree_workspace_setup(workspace, all_locales)?;

    let runner_workspace = WorkspaceInfo {
        root_dir: workspace.workspace_info.root_dir.clone(),
        target_dir: workspace.workspace_info.target_dir.clone(),
        crates: workspace.valid.clone(),
    };

    let _runner_lock =
        crate::generation::acquire_monolithic_runner_lock(&runner_workspace.root_dir)
            .map_err(|error| CliError::Other(error.to_string()))?;

    crate::generation::prepare_monolithic_runner_crate(&runner_workspace)
        .map_err(|error| CliError::Other(error.to_string()))?;

    let temp_store =
        es_fluent_runner::RunnerMetadataStore::temp_for_workspace(&runner_workspace.root_dir);
    let executor = MonolithicExecutor::new(&runner_workspace);
    let mut indexes = HashMap::new();

    for krate in &workspace.valid {
        executor
            .execute_request(&krate.check_request(), false)
            .map_err(|error| CliError::Other(error.to_string()))?;

        let inventory = temp_store
            .read_inventory(&krate.name)
            .map_err(|error| CliError::Other(error.to_string()))?;
        indexes.insert(
            krate.name.to_string(),
            RustLinkIndex::from_inventory(&krate.manifest_dir, inventory),
        );
    }

    Ok(indexes)
}
