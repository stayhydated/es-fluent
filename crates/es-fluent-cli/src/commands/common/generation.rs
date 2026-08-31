use crate::{
    core::{CrateInfo, GenerateResult, GenerationAction, WorkspaceInfo},
    generation::MonolithicExecutor,
    utils::ui,
};

/// Run generation-like work using the monolithic temp crate approach.
///
/// This prepares a single temp crate at workspace root that links the requested
/// crates, then runs the binary sequentially for each crate. Much faster on
/// subsequent runs.
///
/// If `force_run` is true, the staleness check is skipped and the runner is always rebuilt.
pub fn run_generation_for_crates(
    workspace: &WorkspaceInfo,
    crates: &[CrateInfo],
    action: &GenerationAction,
    force_run: bool,
    show_progress: bool,
) -> Vec<GenerateResult> {
    run_generation_for_crates_with_transaction(
        workspace,
        crates,
        action,
        force_run,
        show_progress,
        es_fluent_runner::FileTransaction::default(),
    )
}

pub(crate) fn run_generation_for_crates_with_transaction(
    workspace: &WorkspaceInfo,
    crates: &[CrateInfo],
    action: &GenerationAction,
    force_run: bool,
    show_progress: bool,
    additional_transaction: es_fluent_runner::FileTransaction,
) -> Vec<GenerateResult> {
    let runner_workspace = WorkspaceInfo {
        root_dir: workspace.root_dir.clone(),
        target_dir: workspace.target_dir.clone(),
        crates: crates.to_vec(),
    };

    let _runner_lock =
        match crate::generation::acquire_monolithic_runner_lock(&runner_workspace.root_dir) {
            Ok(lock) => lock,
            Err(e) => {
                return crates
                    .iter()
                    .map(|k| {
                        GenerateResult::failure(
                            k.name.clone(),
                            std::time::Duration::ZERO,
                            e.to_string(),
                        )
                    })
                    .collect();
            },
        };

    if let Err(e) = crate::generation::prepare_monolithic_runner_crate(&runner_workspace) {
        return crates
            .iter()
            .map(|k| {
                GenerateResult::failure(k.name.clone(), std::time::Duration::ZERO, e.to_string())
            })
            .collect();
    }

    let executor = MonolithicExecutor::new(&runner_workspace);
    let pb = if show_progress {
        ui::Ui::create_progress_bar(crates.len() as u64, "Processing crates...")
    } else {
        indicatif::ProgressBar::hidden()
    };

    let planned = crates
        .iter()
        .map(|krate| {
            let result = executor.plan_generation_action(krate, action, force_run);
            pb.inc(1);
            result
        })
        .collect::<Vec<_>>();
    let (mut results, transactions): (Vec<_>, Vec<_>) = planned.into_iter().unzip();

    if results.iter().any(|result| result.error.is_some()) || action.is_dry_run() {
        return results;
    }

    let mut transaction = additional_transaction;
    let transaction_result = transactions
        .into_iter()
        .try_for_each(|planned| transaction.extend(planned))
        .and_then(|()| transaction.commit().map(|_| ()));
    if let Err(error) = transaction_result {
        let message = format!("transaction failed: {error}");
        for result in &mut results {
            result.error = Some(message.clone());
            result.changed = false;
        }
    }

    results
}
