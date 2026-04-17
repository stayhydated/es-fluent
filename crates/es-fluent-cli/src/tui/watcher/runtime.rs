use super::events::{PathToCrateMap, build_path_to_crate, process_file_events};
use super::generation::{compute_watch_inputs_hash, spawn_generation};
use crate::core::{CrateInfo, CrateState, FluentParseMode, GenerateResult, WorkspaceInfo};
use crate::tui::{Message, TuiApp};
use crossbeam_channel::{Receiver, Sender, unbounded};
use notify_debouncer_full::DebouncedEvent;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(super) struct WatchRuntime<'a> {
    workspace: Arc<WorkspaceInfo>,
    mode: FluentParseMode,
    valid_crates: Vec<&'a CrateInfo>,
    crates_by_name: HashMap<String, &'a CrateInfo>,
    path_to_crate: PathToCrateMap,
    observed_hashes: HashMap<String, String>,
    active_generation_hashes: HashMap<String, String>,
    dirty_generating_crates: HashSet<String>,
    result_tx: Sender<GenerateResult>,
    result_rx: Receiver<GenerateResult>,
}

impl<'a> WatchRuntime<'a> {
    pub(super) fn new(
        crates: &'a [CrateInfo],
        workspace: &WorkspaceInfo,
        mode: &FluentParseMode,
    ) -> Self {
        let valid_crates: Vec<_> = crates.iter().filter(|krate| krate.has_lib_rs).collect();
        let path_to_crate = build_path_to_crate(&valid_crates);
        let mut crates_by_name = HashMap::new();
        let mut observed_hashes = HashMap::new();

        for krate in &valid_crates {
            crates_by_name.insert(krate.name.clone(), *krate);
            observed_hashes.insert(
                krate.name.clone(),
                compute_watch_inputs_hash(
                    &krate.manifest_dir,
                    &krate.src_dir,
                    &krate.i18n_config_path,
                ),
            );
        }

        let (result_tx, result_rx) = unbounded();

        Self {
            workspace: Arc::new(workspace.clone()),
            mode: mode.clone(),
            valid_crates,
            crates_by_name,
            path_to_crate,
            observed_hashes,
            active_generation_hashes: HashMap::new(),
            dirty_generating_crates: HashSet::new(),
            result_tx,
            result_rx,
        }
    }

    pub(super) fn valid_crates(&self) -> &[&'a CrateInfo] {
        &self.valid_crates
    }

    pub(super) fn spawn_initial_generations(&mut self, app: &mut TuiApp<'_>) -> bool {
        if self.valid_crates.is_empty() {
            return false;
        }

        for krate in self.valid_crates.clone() {
            self.start_generation(app, krate, false);
        }

        true
    }

    pub(super) fn handle_generation_results(&mut self, app: &mut TuiApp<'_>) {
        while let Ok(result) = self.result_rx.try_recv() {
            let crate_name = result.name.clone();
            let rerun_needed = self.finish_generation(&crate_name);
            app.update(Message::GenerationComplete { result });

            if rerun_needed && let Some(krate) = self.crates_by_name.get(&crate_name).copied() {
                self.start_generation(app, krate, false);
            }
        }
    }

    pub(super) fn handle_file_events(&mut self, app: &mut TuiApp<'_>, events: &[DebouncedEvent]) {
        for crate_name in process_file_events(events, &self.path_to_crate) {
            let Some(krate) = self.crates_by_name.get(&crate_name).copied() else {
                continue;
            };

            let new_hash = compute_watch_inputs_hash(
                &krate.manifest_dir,
                &krate.src_dir,
                &krate.i18n_config_path,
            );
            if !self.observe_hash(&crate_name, new_hash) {
                continue;
            }

            if matches!(app.states.get(&crate_name), Some(CrateState::Generating))
                || self.active_generation_hashes.contains_key(&crate_name)
            {
                continue;
            }

            self.start_generation(app, krate, true);
        }
    }

    fn start_generation(&mut self, app: &mut TuiApp<'_>, krate: &CrateInfo, report_change: bool) {
        if report_change {
            app.update(Message::FileChanged {
                crate_name: krate.name.clone(),
            });
        }

        self.begin_generation(&krate.name);
        app.update(Message::GenerationStarted {
            crate_name: krate.name.clone(),
        });
        self.spawn_for(krate);
    }

    fn spawn_for(&self, krate: &CrateInfo) {
        spawn_generation(
            krate.clone(),
            self.workspace.clone(),
            self.mode.clone(),
            self.result_tx.clone(),
        );
    }

    fn observe_hash(&mut self, crate_name: &str, new_hash: String) -> bool {
        if self.observed_hashes.get(crate_name) == Some(&new_hash) {
            return false;
        }

        self.observed_hashes
            .insert(crate_name.to_string(), new_hash.clone());

        if self
            .active_generation_hashes
            .get(crate_name)
            .is_some_and(|active_hash| active_hash != &new_hash)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CrateInfo, FluentParseMode, WorkspaceInfo};
    use std::path::PathBuf;

    fn test_crate() -> CrateInfo {
        CrateInfo {
            name: "crate-a".to_string(),
            manifest_dir: PathBuf::from("/tmp/test"),
            src_dir: PathBuf::from("/tmp/test/src"),
            i18n_config_path: PathBuf::from("/tmp/test/i18n.toml"),
            ftl_output_dir: PathBuf::from("/tmp/test/i18n/en"),
            has_lib_rs: true,
            fluent_features: Vec::new(),
        }
    }

    fn test_runtime<'a>(krate: &'a CrateInfo) -> WatchRuntime<'a> {
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
            .insert(krate.name.clone(), "hash-a".to_string());

        runtime.begin_generation(&krate.name);
        assert_eq!(
            runtime.active_generation_hashes.get(&krate.name),
            Some(&"hash-a".to_string())
        );

        assert!(runtime.observe_hash(&krate.name, "hash-b".to_string()));
        assert!(runtime.dirty_generating_crates.contains(&krate.name));
    }

    #[test]
    fn finish_generation_requests_rerun_when_latest_hash_differs_from_started_hash() {
        let krate = test_crate();
        let mut runtime = test_runtime(&krate);
        runtime
            .observed_hashes
            .insert(krate.name.clone(), "hash-a".to_string());

        runtime.begin_generation(&krate.name);
        runtime.observe_hash(&krate.name, "hash-b".to_string());

        assert!(runtime.finish_generation(&krate.name));
        assert!(!runtime.dirty_generating_crates.contains(&krate.name));
        assert!(!runtime.active_generation_hashes.contains_key(&krate.name));
    }
}
