use crate::core::CrateInfo;
use indexmap::IndexMap;
use notify_debouncer_full::DebouncedEvent;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub(super) struct PathToCrateMap {
    workspace_root: PathBuf,
    workspace_crates: Vec<String>,
    manifest_dirs: Vec<(PathBuf, String)>,
    src_dirs: Vec<SourceDirMatch>,
    build_sources: Vec<(PathBuf, String)>,
    build_source_dirs: Vec<(PathBuf, String)>,
    missing_default_build_targets: Vec<(PathBuf, String)>,
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
    let (build_sources, build_source_dirs) = build_source_entries(valid_crates);

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
        build_sources,
        build_source_dirs,
        missing_default_build_targets: missing_default_build_target_entries(valid_crates),
        i18n_configs: valid_crates
            .iter()
            .map(|krate| (krate.i18n_config_path.to_path_buf(), krate.name.to_string()))
            .collect(),
    }
}

fn missing_default_build_target_entries(valid_crates: &[&CrateInfo]) -> Vec<(PathBuf, String)> {
    valid_crates
        .iter()
        .filter(|krate| krate.custom_build_target_path.is_none())
        .map(|krate| (krate.manifest_dir.join("build.rs"), krate.name.to_string()))
        .collect()
}

fn build_source_entries(
    valid_crates: &[&CrateInfo],
) -> (Vec<(PathBuf, String)>, Vec<(PathBuf, String)>) {
    let mut build_sources = Vec::new();
    let mut build_source_dirs = Vec::new();

    for krate in valid_crates {
        let Some(build_target) = &krate.custom_build_target_path else {
            continue;
        };
        let graph =
            crate::source_inspector::reachable_source_graph(build_target, &krate.manifest_dir);
        let crate_name = krate.name.to_string();

        if !graph.indeterminate_reasons.is_empty() {
            build_source_dirs.push((krate.manifest_dir.to_path_buf(), crate_name.clone()));
        }

        build_sources.extend(
            graph
                .paths
                .iter()
                .cloned()
                .map(|path| (path, crate_name.clone())),
        );
        build_source_dirs.extend(graph.paths.into_iter().filter_map(|source| {
            if source.starts_with(&krate.src_dir)
                || source.parent() == Some(krate.manifest_dir.as_path())
            {
                return None;
            }
            source
                .parent()
                .map(|parent| (parent.to_path_buf(), crate_name.clone()))
        }));
    }

    (build_sources, build_source_dirs)
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

            if path.file_name().is_some_and(|name| name == "Cargo.toml")
                && let Some(crate_name) = path_to_crate.match_manifest_path(path)
            {
                affected.insert(crate_name.to_string(), ());
                continue;
            }

            let mut matched_build_source = false;
            for crate_name in path_to_crate.match_build_sources(path) {
                affected.insert(crate_name.to_string(), ());
                matched_build_source = true;
            }
            if matched_build_source {
                continue;
            }

            if let Some(crate_name) = path_to_crate.match_missing_default_build_target(path) {
                affected.insert(crate_name.to_string(), ());
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "rs") {
                let mut matched_build_source_dir = false;
                for crate_name in path_to_crate.match_build_source_dirs(path) {
                    affected.insert(crate_name.to_string(), ());
                    matched_build_source_dir = true;
                }
                if matched_build_source_dir {
                    continue;
                }
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
    pub(super) fn refresh_build_sources(&mut self, valid_crates: &[&CrateInfo]) {
        let (build_sources, build_source_dirs) = build_source_entries(valid_crates);
        self.build_sources = build_sources;
        self.build_source_dirs = build_source_dirs;
        self.missing_default_build_targets = missing_default_build_target_entries(valid_crates);
    }

    pub(super) fn build_source_watch_dirs(&self) -> BTreeSet<PathBuf> {
        self.build_source_dirs
            .iter()
            .map(|(directory, _)| directory.clone())
            .collect()
    }

    pub(super) fn should_refresh_build_sources(&self, events: &[DebouncedEvent]) -> bool {
        events.iter().any(|event| {
            event
                .paths
                .iter()
                .any(|path| self.is_build_source_event(path) || self.is_manifest_event(path))
        })
    }

    pub(super) fn has_rediscovery_event(&self, events: &[DebouncedEvent]) -> bool {
        events.iter().flat_map(|event| &event.paths).any(|path| {
            self.is_manifest_event(path) || self.match_missing_default_build_target(path).is_some()
        })
    }

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

    fn match_build_sources<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a str> + 'a {
        self.build_sources
            .iter()
            .filter(move |(source, _)| source == path)
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn match_missing_default_build_target(&self, path: &Path) -> Option<&str> {
        self.missing_default_build_targets
            .iter()
            .find(|(target, _)| target == path)
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn match_build_source_dirs<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a str> + 'a {
        self.build_source_dirs
            .iter()
            .filter(move |(directory, _)| {
                path.starts_with(directory) && !self.is_generated_build_path(path)
            })
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn is_manifest_event(&self, path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "Cargo.toml")
            && (self.is_workspace_root_path(path) || self.match_manifest_path(path).is_some())
    }

    fn is_generated_build_path(&self, path: &Path) -> bool {
        self.manifest_dirs.iter().any(|(manifest_dir, _)| {
            path.starts_with(manifest_dir.join("target"))
                || path.starts_with(manifest_dir.join(".es-fluent"))
        })
    }

    fn is_build_source_event(&self, path: &Path) -> bool {
        self.match_build_sources(path).next().is_some()
            || self.match_missing_default_build_target(path).is_some()
            || self
                .build_source_dirs
                .iter()
                .any(|(directory, _)| path.starts_with(directory))
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
