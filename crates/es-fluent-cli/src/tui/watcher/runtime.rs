use super::events::{PathToCrateMap, build_path_to_crate, process_file_events};
use super::generation::{compute_src_hash, spawn_generation};
use crate::core::{CrateInfo, CrateState, FluentParseMode, GenerateResult, WorkspaceInfo};
use crate::tui::{Message, TuiApp};
use notify_debouncer_full::DebouncedEvent;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

pub(super) struct WatchRuntime<'a> {
    workspace: Arc<WorkspaceInfo>,
    mode: FluentParseMode,
    valid_crates: Vec<&'a CrateInfo>,
    crates_by_name: HashMap<String, &'a CrateInfo>,
    path_to_crate: PathToCrateMap,
    src_hashes: HashMap<String, String>,
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
        let mut src_hashes = HashMap::new();

        for krate in &valid_crates {
            crates_by_name.insert(krate.name.clone(), *krate);
            src_hashes.insert(
                krate.name.clone(),
                compute_src_hash(&krate.src_dir, &krate.i18n_config_path),
            );
        }

        let (result_tx, result_rx) = mpsc::channel();

        Self {
            workspace: Arc::new(workspace.clone()),
            mode: mode.clone(),
            valid_crates,
            crates_by_name,
            path_to_crate,
            src_hashes,
            result_tx,
            result_rx,
        }
    }

    pub(super) fn valid_crates(&self) -> &[&'a CrateInfo] {
        &self.valid_crates
    }

    pub(super) fn spawn_initial_generations(&self, app: &mut TuiApp<'_>) -> bool {
        if self.valid_crates.is_empty() {
            return false;
        }

        for krate in &self.valid_crates {
            app.update(Message::GenerationStarted {
                crate_name: krate.name.clone(),
            });
            self.spawn_for(krate);
        }

        true
    }

    pub(super) fn handle_generation_results(&mut self, app: &mut TuiApp<'_>) {
        while let Ok(result) = self.result_rx.try_recv() {
            self.refresh_hash(&result.name);
            app.update(Message::GenerationComplete { result });
        }
    }

    pub(super) fn handle_file_events(&mut self, app: &mut TuiApp<'_>, events: &[DebouncedEvent]) {
        for crate_name in process_file_events(events, &self.path_to_crate) {
            let Some(krate) = self.crates_by_name.get(&crate_name).copied() else {
                continue;
            };

            if matches!(app.states.get(&crate_name), Some(CrateState::Generating)) {
                continue;
            }

            let new_hash = compute_src_hash(&krate.src_dir, &krate.i18n_config_path);
            if self.src_hashes.get(&crate_name) == Some(&new_hash) {
                continue;
            }

            app.update(Message::FileChanged {
                crate_name: krate.name.clone(),
            });
            app.update(Message::GenerationStarted {
                crate_name: krate.name.clone(),
            });
            self.spawn_for(krate);
        }
    }

    fn spawn_for(&self, krate: &CrateInfo) {
        spawn_generation(
            krate.clone(),
            self.workspace.clone(),
            self.mode.clone(),
            self.result_tx.clone(),
        );
    }

    fn refresh_hash(&mut self, crate_name: &str) {
        let Some(krate) = self.crates_by_name.get(crate_name).copied() else {
            return;
        };

        self.src_hashes.insert(
            crate_name.to_string(),
            compute_src_hash(&krate.src_dir, &krate.i18n_config_path),
        );
    }
}
