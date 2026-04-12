use crate::core::{CrateInfo, FluentParseMode, GenerateResult, GenerationAction, WorkspaceInfo};
use crate::generation::execute_generation_action_monolithic;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

/// Compute a hash of all .rs files in the src directory and the i18n.toml file using blake3.
pub(super) fn compute_src_hash(src_dir: &Path, i18n_config_path: &Path) -> String {
    crate::generation::cache::compute_content_hash(src_dir, Some(i18n_config_path))
}

/// Spawn a thread to generate for a single crate using the monolithic approach.
pub(super) fn spawn_generation(
    krate: CrateInfo,
    workspace: Arc<WorkspaceInfo>,
    mode: FluentParseMode,
    result_tx: Sender<GenerateResult>,
) {
    thread::spawn(move || {
        let result = execute_generation_action_monolithic(
            &krate,
            &workspace,
            &GenerationAction::Generate {
                mode,
                dry_run: false,
            },
            false,
        );

        let _ = result_tx.send(result);
    });
}
