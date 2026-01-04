//! Clean command implementation.

use crate::commands::{
    WorkspaceArgs, WorkspaceCrates, parallel_generate, render_generation_results,
};
use crate::core::{CliError, GenerationAction};
use crate::utils::ui;
use clap::Parser;
use colored::Colorize as _;

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

    let action = GenerationAction::Clean {
        all_locales: args.all,
        dry_run: args.dry_run,
    };

    let results = parallel_generate(&workspace.valid, &action);
    let has_errors = render_generation_results(
        &results,
        |result| {
            if args.dry_run {
                if let Some(output) = &result.output {
                    print!("{}", output);
                } else if result.changed {
                    println!(
                        "{} {} ({} resources)",
                        format!("{} would be cleaned in", result.name).yellow(),
                        humantime::format_duration(result.duration)
                            .to_string()
                            .green(),
                        result.resource_count.to_string().cyan()
                    );
                } else {
                    println!("{} {}", "Unchanged:".dimmed(), result.name.bold());
                }
            } else if result.changed {
                ui::print_cleaned(&result.name, result.duration, result.resource_count);
            } else {
                println!("{} {}", "Unchanged:".dimmed(), result.name.bold());
            }
        },
        |result| ui::print_generation_error(&result.name, result.error.as_ref().unwrap()),
    );

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}
