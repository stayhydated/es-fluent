//! Generate command implementation.

use crate::commands::{
    WorkspaceArgs, WorkspaceCrates, parallel_generate, render_generation_results,
};
use crate::core::{CliError, FluentParseMode};
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
}

/// Run the generate command.
pub fn run_generate(args: GenerateArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    for krate in &workspace.valid {
        ui::print_generating(&krate.name);
    }

    let results = parallel_generate(&workspace.valid, &args.mode);
    let has_errors = render_generation_results(
        &results,
        |result| ui::print_generated(&result.name, result.duration, result.resource_count),
        |result| ui::print_generation_error(&result.name, result.error.as_ref().unwrap()),
    );

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
