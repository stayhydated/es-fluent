//! Generate command implementation.

use crate::commands::{
    GenerationVerb, WorkspaceArgs, WorkspaceCrates, parallel_generate,
    render_generation_results_with_dry_run,
};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use crate::utils::ui;
use clap::Parser;

/// Arguments for the generate command.
#[derive(Parser)]
pub struct GenerateArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,

    /// Dry run - show what would be generated without making changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Force rebuild of the runner, ignoring the staleness cache.
    #[arg(long)]
    pub force_run: bool,
}

/// Run the generate command.
pub fn run_generate(args: GenerateArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    let results = parallel_generate(
        &workspace.workspace_info,
        &workspace.valid,
        &GenerationAction::Generate {
            mode: args.mode,
            dry_run: args.dry_run,
        },
        args.force_run,
    );
    let has_errors =
        render_generation_results_with_dry_run(&results, args.dry_run, GenerationVerb::Generate);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
