use super::events::PathToCrateMap;
use crate::core::{CrateInfo, CrateState, FluentParseMode, GenerateResult, WorkspaceInfo};
use crate::tui::{Message, TuiApp};
use crossbeam_channel::{Receiver, Sender};
use notify_debouncer_full::DebouncedEvent;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct BuildSourceWatchUpdate {
    pub(super) added: Vec<PathBuf>,
    pub(super) rearmed: Vec<PathBuf>,
    pub(super) removed: Vec<PathBuf>,
}

pub(super) struct WatchRuntime {
    workspace: Arc<WorkspaceInfo>,
    mode: FluentParseMode,
    valid_crates: Vec<CrateInfo>,
    crates_by_name: HashMap<String, CrateInfo>,
    path_to_crate: PathToCrateMap,
    custom_build_dirs: BTreeSet<PathBuf>,
    observed_hashes: HashMap<String, Option<String>>,
    active_generation_hashes: HashMap<String, Option<String>>,
    dirty_generating_crates: HashSet<String>,
    generation_handles: HashMap<String, JoinHandle<()>>,
    result_tx: Sender<GenerateResult>,
    result_rx: Receiver<GenerateResult>,
}

impl WatchRuntime {
    pub(super) fn new(
        crates: &[CrateInfo],
        workspace: &WorkspaceInfo,
        mode: &FluentParseMode,
    ) -> Self {
        let valid_crates = crates
            .iter()
            .filter(|krate| krate.has_lib_rs)
            .cloned()
            .collect::<Vec<_>>();
        let valid_crate_refs = valid_crates.iter().collect::<Vec<_>>();
        let path_to_crate =
            super::events::build_path_to_crate(&valid_crate_refs, &workspace.root_dir);
        let custom_build_dirs = path_to_crate.build_source_watch_dirs();
        let mut crates_by_name = HashMap::new();
        let mut observed_hashes = HashMap::new();

        for krate in &valid_crates {
            crates_by_name.insert(krate.name.to_string(), krate.clone());
            observed_hashes.insert(
                krate.name.to_string(),
                super::generation::compute_watch_inputs_hash(
                    &krate.manifest_dir,
                    &krate.src_dir,
                    &krate.i18n_config_path,
                    krate.custom_build_target_path.as_deref(),
                ),
            );
        }

        let (result_tx, result_rx) = crossbeam_channel::unbounded();
        let runner_crates = valid_crates.to_vec();

        Self {
            workspace: Arc::new(super::workspace_for_crates(workspace, &runner_crates)),
            mode: *mode,
            valid_crates,
            crates_by_name,
            path_to_crate,
            custom_build_dirs,
            observed_hashes,
            active_generation_hashes: HashMap::new(),
            dirty_generating_crates: HashSet::new(),
            generation_handles: HashMap::new(),
            result_tx,
            result_rx,
        }
    }

    pub(super) fn valid_crates(&self) -> Vec<&CrateInfo> {
        self.valid_crates.iter().collect()
    }

    pub(super) fn refresh_build_sources_if_needed(
        &mut self,
        events: &[DebouncedEvent],
    ) -> anyhow::Result<Option<BuildSourceWatchUpdate>> {
        if !self.path_to_crate.should_refresh_build_sources(events) {
            return Ok(None);
        }

        if self.path_to_crate.has_rediscovery_event(events) {
            self.rediscover_custom_build_targets()?;
        }
        let valid_crate_refs = self.valid_crates.iter().collect::<Vec<_>>();
        self.path_to_crate.refresh_build_sources(&valid_crate_refs);
        let mut refreshed_dirs = self.path_to_crate.build_source_watch_dirs();
        refreshed_dirs.extend(
            self.custom_build_dirs
                .iter()
                .filter(|directory| {
                    self.valid_crates
                        .iter()
                        .any(|krate| krate.manifest_dir.as_path() == directory.as_path())
                })
                .cloned(),
        );
        let removed = self
            .custom_build_dirs
            .difference(&refreshed_dirs)
            .cloned()
            .collect::<BTreeSet<_>>();
        let update = BuildSourceWatchUpdate {
            added: refreshed_dirs
                .difference(&self.custom_build_dirs)
                .cloned()
                .collect(),
            rearmed: refreshed_dirs
                .iter()
                .filter(|directory| removed.iter().any(|removed| directory.starts_with(removed)))
                .cloned()
                .collect(),
            removed: removed.into_iter().collect(),
        };
        self.custom_build_dirs = refreshed_dirs;
        Ok(Some(update))
    }

    pub(super) fn affected_crates_for_events(&self, events: &[DebouncedEvent]) -> Vec<String> {
        super::events::process_file_events(events, &self.path_to_crate)
    }

    #[cfg(test)]
    pub(super) fn observed_hash(&self, crate_name: &str) -> Option<&str> {
        self.observed_hashes
            .get(crate_name)
            .and_then(Option::as_deref)
    }

    pub(super) fn spawn_initial_generations(&mut self, app: &mut TuiApp<'_>) -> bool {
        if self.valid_crates.is_empty() {
            return false;
        }

        for krate in self.valid_crates.clone() {
            self.start_generation(app, &krate, false);
        }

        true
    }

    pub(super) fn handle_generation_results(&mut self, app: &mut TuiApp<'_>) {
        while let Ok(result) = self.result_rx.try_recv() {
            let crate_name = result.name.clone();
            if let Some(handle) = self.generation_handles.remove(crate_name.as_str())
                && handle.join().is_err()
            {
                app.update(Message::WatchError {
                    error: format!("generation thread panicked for {crate_name}"),
                });
            }
            self.apply_generation_result(app, result);
        }
    }

    pub(super) fn finish_pending_generations(
        &mut self,
        app: &mut TuiApp<'_>,
    ) -> anyhow::Result<()> {
        while !self.generation_handles.is_empty() {
            let handles = std::mem::take(&mut self.generation_handles);
            let mut completed_count = 0usize;
            let mut panicked_crates = Vec::new();

            for (crate_name, handle) in handles {
                if handle.join().is_ok() {
                    completed_count += 1;
                } else {
                    self.finish_generation(crate_name.as_str());
                    app.update(Message::WatchError {
                        error: format!("generation thread panicked for {crate_name}"),
                    });
                    panicked_crates.push(crate_name);
                }
            }

            for _ in 0..completed_count {
                let result = self.result_rx.recv().map_err(|_| {
                    anyhow::anyhow!("generation result channel closed during watch shutdown")
                })?;
                self.apply_generation_result(app, result);
            }

            if !panicked_crates.is_empty() {
                anyhow::bail!(
                    "generation thread panicked for {}",
                    panicked_crates.join(", ")
                );
            }
        }

        Ok(())
    }

    pub(super) fn handle_affected_crates(
        &mut self,
        app: &mut TuiApp<'_>,
        crate_names: impl IntoIterator<Item = String>,
    ) {
        for crate_name in crate_names {
            let Some(krate) = self.crates_by_name.get(&crate_name).cloned() else {
                continue;
            };

            let new_hash = super::generation::compute_watch_inputs_hash(
                &krate.manifest_dir,
                &krate.src_dir,
                &krate.i18n_config_path,
                krate.custom_build_target_path.as_deref(),
            );
            if !self.observe_hash(&crate_name, new_hash) {
                continue;
            }

            if matches!(app.states.get(&crate_name), Some(CrateState::Generating))
                || self.active_generation_hashes.contains_key(&crate_name)
            {
                continue;
            }

            self.start_generation(app, &krate, true);
        }
    }

    fn start_generation(&mut self, app: &mut TuiApp<'_>, krate: &CrateInfo, report_change: bool) {
        if report_change {
            app.update(Message::FileChanged {
                crate_name: krate.name.to_string(),
            });
        }

        self.begin_generation(krate.name.as_str());
        app.update(Message::GenerationStarted {
            crate_name: krate.name.to_string(),
        });
        self.spawn_for(krate);
    }

    fn spawn_for(&mut self, krate: &CrateInfo) {
        let handle = super::generation::spawn_generation(
            krate.clone(),
            self.workspace.clone(),
            self.mode,
            self.result_tx.clone(),
        );
        self.generation_handles
            .insert(krate.name.to_string(), handle);
    }

    fn apply_generation_result(&mut self, app: &mut TuiApp<'_>, result: GenerateResult) {
        let crate_name = result.name.clone();
        let rerun_needed = self.finish_generation(crate_name.as_str());
        app.update(Message::GenerationComplete { result });

        if rerun_needed && let Some(krate) = self.crates_by_name.get(crate_name.as_str()).cloned() {
            self.start_generation(app, &krate, false);
        }
    }

    fn observe_hash(&mut self, crate_name: &str, new_hash: Option<String>) -> bool {
        if new_hash.is_some() && self.observed_hashes.get(crate_name) == Some(&new_hash) {
            return false;
        }

        self.observed_hashes
            .insert(crate_name.to_string(), new_hash.clone());

        if self
            .active_generation_hashes
            .get(crate_name)
            .is_some_and(|active_hash| new_hash.is_none() || active_hash != &new_hash)
        {
            self.dirty_generating_crates.insert(crate_name.to_string());
        }

        true
    }

    fn begin_generation(&mut self, crate_name: &str) {
        if let Some(current_hash) = self.observed_hashes.get(crate_name).cloned() {
            self.active_generation_hashes
                .insert(crate_name.to_string(), current_hash);
        }
        self.dirty_generating_crates.remove(crate_name);
    }

    fn finish_generation(&mut self, crate_name: &str) -> bool {
        let active_hash = self.active_generation_hashes.remove(crate_name);
        let current_hash = self.observed_hashes.get(crate_name);
        let was_marked_dirty = self.dirty_generating_crates.remove(crate_name);

        match (active_hash, current_hash) {
            (Some(active_hash), Some(current_hash)) => {
                was_marked_dirty || &active_hash != current_hash
            },
            _ => false,
        }
    }

    fn rediscover_custom_build_targets(&mut self) -> anyhow::Result<()> {
        let selected_packages = self
            .valid_crates
            .iter()
            .map(|krate| krate.name.to_string())
            .collect::<Vec<_>>();
        let discovered = crate::utils::discover_workspace_scoped(
            &self.workspace.root_dir,
            crate::utils::DiscoveryScope::Packages(&selected_packages),
        )?;
        for krate in &mut self.valid_crates {
            if let Some(refreshed) = discovered
                .crates
                .iter()
                .find(|candidate| candidate.name == krate.name)
            {
                krate.custom_build_target_path = refreshed.custom_build_target_path.clone();
            }
        }
        self.crates_by_name = self
            .valid_crates
            .iter()
            .cloned()
            .map(|krate| (krate.name.to_string(), krate))
            .collect();
        self.workspace = Arc::new(WorkspaceInfo {
            root_dir: self.workspace.root_dir.clone(),
            target_dir: discovered.target_dir,
            crates: self.valid_crates.clone(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
    use std::path::PathBuf;

    fn test_crate() -> CrateInfo {
        CrateInfo {
            name: es_fluent_runner::PackageName::try_new("crate-a").expect("valid package name"),
            manifest_dir: crate::core::ManifestDir::from_discovered(PathBuf::from("/tmp/test")),
            src_dir: crate::core::SourceDir::from_discovered(PathBuf::from("/tmp/test/src")),
            library_target_path: None,
            custom_build_target_path: None,
            i18n_config_path: crate::core::DiscoveredI18nConfigPath::from_discovered(
                PathBuf::from("/tmp/test/i18n.toml"),
            ),
            ftl_output_dir: crate::core::DiscoveredFtlOutputDir::from_discovered(PathBuf::from(
                "/tmp/test/i18n/en",
            )),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    fn test_runtime(krate: &CrateInfo) -> WatchRuntime {
        let workspace = WorkspaceInfo {
            root_dir: PathBuf::from("/tmp/test"),
            target_dir: PathBuf::from("/tmp/test/target"),
            crates: vec![krate.clone()],
        };

        WatchRuntime::new(
            std::slice::from_ref(krate),
            &workspace,
            &FluentParseMode::default(),
        )
    }

    #[test]
    fn observe_hash_marks_generating_crate_dirty_when_content_changes_mid_run() {
        let krate = test_crate();
        let mut runtime = test_runtime(&krate);
        runtime
            .observed_hashes
            .insert(krate.name.to_string(), Some("hash-a".to_string()));

        runtime.begin_generation(krate.name.as_str());
        assert_eq!(
            runtime.active_generation_hashes.get(krate.name.as_str()),
            Some(&Some("hash-a".to_string()))
        );

        assert!(runtime.observe_hash(krate.name.as_str(), Some("hash-b".to_string())));
        assert!(
            runtime
                .dirty_generating_crates
                .contains(krate.name.as_str())
        );
    }

    #[test]
    fn finish_generation_requests_rerun_when_latest_hash_differs_from_started_hash() {
        let krate = test_crate();
        let mut runtime = test_runtime(&krate);
        runtime
            .observed_hashes
            .insert(krate.name.to_string(), Some("hash-a".to_string()));

        runtime.begin_generation(krate.name.as_str());
        runtime.observe_hash(krate.name.as_str(), Some("hash-b".to_string()));

        assert!(runtime.finish_generation(krate.name.as_str()));
        assert!(
            !runtime
                .dirty_generating_crates
                .contains(krate.name.as_str())
        );
        assert!(
            !runtime
                .active_generation_hashes
                .contains_key(krate.name.as_str())
        );
    }
}
