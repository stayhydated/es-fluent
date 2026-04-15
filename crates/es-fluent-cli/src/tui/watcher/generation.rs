use crate::core::{CrateInfo, FluentParseMode, GenerateResult, GenerationAction, WorkspaceInfo};
use crate::generation::MonolithicExecutor;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

/// Compute a hash of the crate-local inputs that affect watch-mode generation.
pub(super) fn compute_watch_inputs_hash(
    manifest_dir: &Path,
    src_dir: &Path,
    i18n_config_path: &Path,
) -> String {
    crate::generation::cache::compute_crate_inputs_hash(
        manifest_dir,
        src_dir,
        Some(i18n_config_path),
    )
}

/// Spawn a thread to generate for a single crate using the monolithic approach.
pub(super) fn spawn_generation(
    krate: CrateInfo,
    workspace: Arc<WorkspaceInfo>,
    mode: FluentParseMode,
    result_tx: Sender<GenerateResult>,
) {
    thread::spawn(move || {
        let executor = MonolithicExecutor::new(&workspace);
        let result = executor.execute_generation_action(
            &krate,
            &GenerationAction::Generate {
                mode,
                dry_run: false,
            },
            false,
        );

        let _ = result_tx.send(result);
    });
}
