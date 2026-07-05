use crate::core::CrateInfo;
use indexmap::IndexMap;
use notify_debouncer_full::DebouncedEvent;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) struct PathToCrateMap {
    workspace_root: PathBuf,
    workspace_crates: Vec<String>,
    manifest_dirs: Vec<(PathBuf, String)>,
    src_dirs: Vec<SourceDirMatch>,
    i18n_configs: IndexMap<PathBuf, String>,
}

struct SourceDirMatch {
    manifest_dir: PathBuf,
    src_dir: PathBuf,
    crate_name: String,
}

const GENERATED_ROOT_SOURCE_DIRS: &[&str] = &["target"];

pub(super) fn build_path_to_crate(
    valid_crates: &[&CrateInfo],
    workspace_root: &Path,
) -> PathToCrateMap {
    PathToCrateMap {
        workspace_root: workspace_root.to_path_buf(),
        workspace_crates: valid_crates
            .iter()
            .map(|krate| krate.name.to_string())
            .collect(),
        manifest_dirs: valid_crates
            .iter()
            .map(|krate| (krate.manifest_dir.to_path_buf(), krate.name.to_string()))
            .collect(),
        src_dirs: valid_crates
            .iter()
            .map(|krate| SourceDirMatch {
                manifest_dir: krate.manifest_dir.to_path_buf(),
                src_dir: krate.src_dir.to_path_buf(),
                crate_name: krate.name.to_string(),
            })
            .collect(),
        i18n_configs: valid_crates
            .iter()
            .map(|krate| (krate.i18n_config_path.to_path_buf(), krate.name.to_string()))
            .collect(),
    }
}

/// Process file events and return the set of affected crate names.
pub(super) fn process_file_events(
    events: &[DebouncedEvent],
    path_to_crate: &PathToCrateMap,
) -> Vec<String> {
    let mut affected: HashMap<String, ()> = HashMap::new();

    for event in events {
        for path in &event.paths {
            if path.components().any(|c| c.as_os_str() == ".es-fluent") {
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "ftl") {
                continue;
            }

            if path
                .file_name()
                .is_some_and(|name| name == "Cargo.toml" || name == "Cargo.lock")
                && path_to_crate.is_workspace_root_path(path)
            {
                for crate_name in path_to_crate.workspace_crates() {
                    affected.insert(crate_name.to_string(), ());
                }
                continue;
            }

            if path
                .file_name()
                .is_some_and(|name| name == "Cargo.toml" || name == "build.rs")
                && let Some(crate_name) = path_to_crate.match_manifest_path(path)
            {
                affected.insert(crate_name.to_string(), ());
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "rs") {
                if let Some(crate_name) = path_to_crate.match_src_path(path) {
                    affected.insert(crate_name.to_string(), ());
                }
                continue;
            }

            if path.file_name().is_some_and(|name| name == "i18n.toml")
                && let Some(crate_name) = path_to_crate.match_i18n_path(path)
            {
                affected.insert(crate_name.to_string(), ());
            }
        }
    }

    affected.into_keys().collect()
}

impl PathToCrateMap {
    fn is_workspace_root_path(&self, path: &Path) -> bool {
        path.parent() == Some(self.workspace_root.as_path())
    }

    fn workspace_crates(&self) -> impl Iterator<Item = &str> {
        self.workspace_crates.iter().map(String::as_str)
    }

    fn match_manifest_path(&self, path: &Path) -> Option<&str> {
        self.manifest_dirs
            .iter()
            .find(|(manifest_dir, _)| path.parent() == Some(manifest_dir.as_path()))
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn match_src_path(&self, path: &Path) -> Option<&str> {
        self.src_dirs
            .iter()
            .find(|candidate| {
                path.starts_with(&candidate.src_dir)
                    && !candidate.is_generated_root_source_path(path)
            })
            .map(|candidate| candidate.crate_name.as_str())
    }

    fn match_i18n_path(&self, path: &Path) -> Option<&str> {
        self.i18n_configs.get(path).map(String::as_str)
    }
}

impl SourceDirMatch {
    fn is_generated_root_source_path(&self, path: &Path) -> bool {
        if self.src_dir != self.manifest_dir {
            return false;
        }

        let Ok(relative_path) = path.strip_prefix(&self.src_dir) else {
            return false;
        };

        relative_path
            .components()
            .next()
            .and_then(|component| match component {
                std::path::Component::Normal(name) => Some(name),
                _ => None,
            })
            .is_some_and(|name| {
                GENERATED_ROOT_SOURCE_DIRS
                    .iter()
                    .any(|ignored| name == *ignored)
            })
    }
}
