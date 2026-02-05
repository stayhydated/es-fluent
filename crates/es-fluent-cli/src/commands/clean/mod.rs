//! Clean command implementation.

mod orphaned;

use crate::commands::{
    GenerationVerb, WorkspaceArgs, WorkspaceCrates, parallel_generate,
    render_generation_results_with_dry_run,
};
use crate::core::{CliError, GenerationAction};
use crate::utils::ui;
use clap::Parser;

use orphaned::clean_orphaned_files;

/// Arguments for the clean command.
#[derive(Parser)]
pub struct CleanArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Clean all locales, not just the fallback language.
    #[arg(long)]
    pub all: bool,

    /// Dry run - show what would be cleaned without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Force rebuild of the runner, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,

    /// Remove orphaned FTL files that are no longer tied to any types.
    /// This removes files that don't correspond to any registered types
    /// (e.g., when all items are now namespaced or the crate was deleted).
    #[arg(long)]
    pub orphaned: bool,
}

/// Run the clean command.
pub fn run_clean(args: CleanArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    // Handle orphaned file removal first if requested
    if args.orphaned {
        return clean_orphaned_files(&workspace, args.all, args.dry_run);
    }

    let action = GenerationAction::Clean {
        all_locales: args.all,
        dry_run: args.dry_run,
    };

    let results = parallel_generate(
        &workspace.workspace_info,
        &workspace.valid,
        &action,
        args.force_run,
    );
    let has_errors =
        render_generation_results_with_dry_run(&results, args.dry_run, GenerationVerb::Clean);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
