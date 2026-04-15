use crate::core::CrateInfo;
use indexmap::IndexMap;
use notify_debouncer_full::DebouncedEvent;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) struct PathToCrateMap {
    manifest_dirs: Vec<(PathBuf, String)>,
    src_dirs: Vec<(PathBuf, String)>,
    i18n_configs: IndexMap<PathBuf, String>,
}

pub(super) fn build_path_to_crate(valid_crates: &[&CrateInfo]) -> PathToCrateMap {
    PathToCrateMap {
        manifest_dirs: valid_crates
            .iter()
            .map(|krate| (krate.manifest_dir.clone(), krate.name.clone()))
            .collect(),
        src_dirs: valid_crates
            .iter()
            .map(|krate| (krate.src_dir.clone(), krate.name.clone()))
            .collect(),
        i18n_configs: valid_crates
            .iter()
            .map(|krate| (krate.i18n_config_path.clone(), krate.name.clone()))
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
    fn match_manifest_path(&self, path: &Path) -> Option<&str> {
        self.manifest_dirs
            .iter()
            .find(|(manifest_dir, _)| path.parent() == Some(manifest_dir.as_path()))
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn match_src_path(&self, path: &Path) -> Option<&str> {
        self.src_dirs
            .iter()
            .find(|(src_dir, _)| path.starts_with(src_dir))
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn match_i18n_path(&self, path: &Path) -> Option<&str> {
        self.i18n_configs.get(path).map(String::as_str)
    }
}
