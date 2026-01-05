//! Generate command implementation.

use crate::commands::{WorkspaceArgs, WorkspaceCrates, parallel_generate, render_generation_results};
use crate::core::{CliError, FluentParseMode, GenerationAction};
use crate::utils::ui;
use clap::Parser;
use colored::Colorize as _;

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
    );
    let has_errors = render_generation_results(
        &results,
        |result| {
            if args.dry_run {
                if let Some(output) = &result.output {
                    print!("{}", output);
                } else if result.changed {
                    // Fallback if no output captured but marked as changed
                    println!(
                        "{} {} ({} resources)",
                        format!("{} would be generated in", result.name).yellow(),
                        humantime::format_duration(result.duration)
                            .to_string()
                            .green(),
                        result.resource_count.to_string().cyan()
                    );
                } else {
                    println!("{} {}", "Unchanged:".dimmed(), result.name.bold());
                }
            } else if result.changed {
                ui::print_generated(&result.name, result.duration, result.resource_count);
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
