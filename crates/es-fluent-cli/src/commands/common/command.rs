use crate::{
    core::{CliError, GenerationAction},
    utils::ui,
};

use super::{
    GenerationVerb, WorkspaceArgs, WorkspaceCrates, render_generation_results_with_dry_run,
    run_generation_for_crates, validate_generation_paths,
};

/// Execute a generation-like command that uses the monolithic runner.
pub fn run_generation_command(
    workspace_args: WorkspaceArgs,
    action: GenerationAction,
    force_run: bool,
    dry_run: bool,
    verb: GenerationVerb,
) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(workspace_args)?;

    if !workspace.print_discovery(ui::Ui::print_header) {
        return workspace.require_non_empty_selection();
    }
    workspace.require_all_crates_valid()?;
    validate_generation_paths(&workspace.valid, true)?;

    let results = run_generation_for_crates(
        &workspace.workspace_info,
        &workspace.valid,
        &action,
        force_run,
        true,
    );
    let has_errors = render_generation_results_with_dry_run(&results, dry_run, verb);

    if has_errors {
        return Err(CliError::Other(
            "generation command failed; see diagnostics above".to_string(),
        ));
    }

    Ok(())
}
