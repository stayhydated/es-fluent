//! Clean command implementation.

use crate::commands::{
    WorkspaceArgs, WorkspaceCrates, parallel_generate, render_generation_results,
};
use crate::core::{CliError, FluentParseMode};
use crate::utils::ui;
use clap::Parser;

/// Arguments for the clean command.
#[derive(Parser)]
pub struct CleanArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,
}

/// Run the clean command.
pub fn run_clean(args: CleanArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    for krate in &workspace.valid {
        ui::print_cleaning(&krate.name);
    }

    let results = parallel_generate(&workspace.valid, &FluentParseMode::Clean);
    let has_errors = render_generation_results(
        &results,
        |result| ui::print_cleaned(&result.name, result.duration, result.resource_count),
        |result| ui::print_generation_error(&result.name, result.error.as_ref().unwrap()),
    );

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
