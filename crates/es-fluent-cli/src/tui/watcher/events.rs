use crate::core::CrateInfo;
use indexmap::IndexMap;
use notify_debouncer_full::DebouncedEvent;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub(super) struct PathToCrateMap {
    workspace_root: PathBuf,
    target_dir: PathBuf,
    cargo_config_dirs: BTreeSet<PathBuf>,
    cargo_config_paths: BTreeSet<PathBuf>,
    cargo_config_topology_dirs: BTreeSet<PathBuf>,
    cargo_config_topology_paths: BTreeSet<PathBuf>,
    workspace_crates: Vec<String>,
    manifest_dirs: Vec<(PathBuf, String)>,
    src_dirs: Vec<SourceDirMatch>,
    build_sources: Vec<(PathBuf, String)>,
    build_source_dirs: Vec<(PathBuf, String)>,
    missing_default_build_targets: Vec<(PathBuf, String)>,
    i18n_configs: IndexMap<PathBuf, String>,
}

struct SourceDirMatch {
    src_dir: PathBuf,
    crate_name: String,
}

pub(super) fn build_path_to_crate(
    valid_crates: &[&CrateInfo],
    workspace_root: &Path,
    target_dir: &Path,
) -> PathToCrateMap {
    build_path_to_crate_with_cargo_home(
        valid_crates,
        workspace_root,
        target_dir,
        configured_cargo_home(),
    )
}

pub(super) fn build_path_to_crate_with_cargo_home(
    valid_crates: &[&CrateInfo],
    workspace_root: &Path,
    target_dir: &Path,
    cargo_home: Option<PathBuf>,
) -> PathToCrateMap {
    let (build_sources, build_source_dirs) = build_source_entries(valid_crates);
    let cargo_config_candidates = cargo_config_candidates(workspace_root, cargo_home);
    let cargo_config_dirs = cargo_config_candidates
        .iter()
        .filter(|directory| directory.is_dir())
        .cloned()
        .collect::<BTreeSet<_>>();
    let cargo_config_paths = cargo_config_candidates
        .iter()
        .flat_map(|directory| [directory.join("config.toml"), directory.join("config")])
        .collect();
    let (cargo_config_topology_dirs, cargo_config_topology_paths) = cargo_config_candidates
        .iter()
        .filter_map(|directory| cargo_config_topology_watch(directory))
        .unzip();

    PathToCrateMap {
        workspace_root: workspace_root.to_path_buf(),
        target_dir: target_dir.to_path_buf(),
        cargo_config_dirs,
        cargo_config_paths,
        cargo_config_topology_dirs,
        cargo_config_topology_paths,
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

fn configured_cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
}

fn cargo_config_candidates(
    workspace_root: &Path,
    cargo_home: Option<PathBuf>,
) -> BTreeSet<PathBuf> {
    let mut directories = workspace_root
        .ancestors()
        .map(|ancestor| ancestor.join(".cargo"))
        .collect::<BTreeSet<_>>();
    if let Some(cargo_home) = cargo_home {
        let cargo_home = if cargo_home.is_absolute() {
            cargo_home
        } else {
            workspace_root.join(cargo_home)
        };
        directories.insert(cargo_home);
    }
    directories
}

fn cargo_config_topology_watch(candidate: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut missing_suffix = Vec::new();
    let mut current = candidate.parent()?;
    while !current.is_dir() {
        missing_suffix.push(current.file_name()?.to_os_string());
        current = current.parent()?;
    }
    let topology_path = missing_suffix
        .last()
        .map_or_else(|| candidate.to_path_buf(), |name| current.join(name));
    Some((current.to_path_buf(), topology_path))
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

            if path_to_crate.is_cargo_config_event(path) {
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

            for crate_name in path_to_crate.match_build_sources(path) {
                affected.insert(crate_name.to_string(), ());
            }

            if let Some(crate_name) = path_to_crate.match_missing_default_build_target(path) {
                affected.insert(crate_name.to_string(), ());
            }

            for crate_name in path_to_crate.match_build_source_dirs(path) {
                affected.insert(crate_name.to_string(), ());
            }

            if path.extension().is_some_and(|ext| ext == "rs") {
                for crate_name in path_to_crate.match_src_paths(path) {
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
    pub(super) fn refresh_for_crates(&mut self, valid_crates: &[&CrateInfo], target_dir: &Path) {
        let workspace_root = self.workspace_root.clone();
        *self = build_path_to_crate(valid_crates, &workspace_root, target_dir);
    }

    pub(super) fn build_source_watch_dirs(&self) -> BTreeSet<PathBuf> {
        self.build_source_dirs
            .iter()
            .map(|(directory, _)| directory.clone())
            .collect()
    }

    pub(super) fn cargo_config_watch_dirs(&self) -> BTreeSet<PathBuf> {
        self.cargo_config_dirs
            .iter()
            .chain(&self.cargo_config_topology_dirs)
            .cloned()
            .collect()
    }

    pub(super) fn should_refresh_build_sources(&self, events: &[DebouncedEvent]) -> bool {
        events.iter().any(|event| {
            event.paths.iter().any(|path| {
                self.is_build_source_event(path)
                    || self.is_manifest_event(path)
                    || self.is_cargo_config_event(path)
            })
        })
    }

    pub(super) fn has_rediscovery_event(&self, events: &[DebouncedEvent]) -> bool {
        events.iter().flat_map(|event| &event.paths).any(|path| {
            self.is_manifest_event(path)
                || self.is_cargo_config_event(path)
                || self.match_missing_default_build_target(path).is_some()
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
                path.starts_with(directory)
                    && !self.is_generated_build_path_for_directory(path, directory)
            })
            .map(|(_, crate_name)| crate_name.as_str())
    }

    fn is_manifest_event(&self, path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "Cargo.toml")
            && (self.is_workspace_root_path(path) || self.match_manifest_path(path).is_some())
    }

    fn is_cargo_config_event(&self, path: &Path) -> bool {
        self.cargo_config_paths.contains(path) || self.cargo_config_topology_paths.contains(path)
    }

    fn is_target_output_for_directory(&self, path: &Path, directory: &Path) -> bool {
        self.target_dir != directory
            && self.target_dir.starts_with(directory)
            && path.starts_with(&self.target_dir)
    }

    fn is_generated_build_path_for_directory(&self, path: &Path, directory: &Path) -> bool {
        self.is_target_output_for_directory(path, directory)
            || path
                .components()
                .any(|component| component.as_os_str() == ".es-fluent")
    }

    fn is_build_source_event(&self, path: &Path) -> bool {
        self.match_build_sources(path).next().is_some()
            || self.match_missing_default_build_target(path).is_some()
            || self.build_source_dirs.iter().any(|(directory, _)| {
                path.starts_with(directory)
                    && !self.is_generated_build_path_for_directory(path, directory)
            })
    }

    fn match_src_paths<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a str> + 'a {
        self.src_dirs
            .iter()
            .filter(move |candidate| {
                path.starts_with(&candidate.src_dir)
                    && !self.is_target_output_for_directory(path, &candidate.src_dir)
            })
            .map(|candidate| candidate.crate_name.as_str())
    }

    fn match_i18n_path(&self, path: &Path) -> Option<&str> {
        self.i18n_configs.get(path).map(String::as_str)
    }
}
