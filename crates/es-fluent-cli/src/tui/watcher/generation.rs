use crate::core::{CrateInfo, FluentParseMode, GenerateResult, GenerationAction, WorkspaceInfo};
use crate::generation::MonolithicExecutor;
use crossbeam_channel::Sender;
use std::path::Path;
use std::sync::Arc;
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
        let result = match crate::generation::acquire_monolithic_runner_lock(&workspace.root_dir)
            .and_then(|_runner_lock| {
                crate::generation::prepare_monolithic_runner_crate(&workspace)?;
                let executor = MonolithicExecutor::new(&workspace);
                Ok(executor.execute_generation_action(
                    &krate,
                    &GenerationAction::Generate {
                        mode,
                        dry_run: false,
                    },
                    false,
                ))
            }) {
            Ok(result) => result,
            Err(error) => GenerateResult::failure(
                krate.name.clone(),
                std::time::Duration::ZERO,
                error.to_string(),
            ),
        };

        let _ = result_tx.send(result);
    });
}
