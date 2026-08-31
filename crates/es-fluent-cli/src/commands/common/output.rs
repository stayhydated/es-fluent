use crate::{
    core::{CliError, GenerateResult},
    utils::ui,
};
use anstream::{print, println};
use clap::ValueEnum;
use colored::Colorize as _;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl OutputFormat {
    pub fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }

    pub fn print_json<T: Serialize>(self, value: &T) -> Result<(), CliError> {
        if self.is_json() {
            println!(
                "{}",
                serde_json::to_string_pretty(value)
                    .map_err(|error| CliError::Other(error.to_string()))?
            );
        }

        Ok(())
    }
}

/// Render a list of `GenerateResult`s with custom success/error handlers.
///
/// Returns `true` when any errors were encountered.
pub fn render_generation_results(
    results: &[GenerateResult],
    on_success: impl Fn(&GenerateResult),
    on_error: impl Fn(&GenerateResult),
) -> bool {
    let mut has_errors = false;

    for result in results {
        if result.error.is_some() {
            has_errors = true;
            on_error(result);
        } else {
            on_success(result);
        }
    }

    has_errors
}

#[derive(Clone, Copy, Debug)]
pub enum GenerationVerb {
    Generate,
    Clean,
}

impl GenerationVerb {
    pub(super) fn dry_run_label(self) -> &'static str {
        match self {
            GenerationVerb::Generate => "would be generated in",
            GenerationVerb::Clean => "would be cleaned in",
        }
    }

    fn print_changed(self, result: &GenerateResult) {
        match self {
            GenerationVerb::Generate => {
                ui::Ui::print_generated(
                    result.name.as_str(),
                    result.duration,
                    result.resource_count,
                );
            },
            GenerationVerb::Clean => {
                ui::Ui::print_cleaned(result.name.as_str(), result.duration, result.resource_count);
            },
        }
    }
}

/// Render generation-like results with the standard dry-run output.
///
/// Returns `true` when any errors were encountered.
pub fn render_generation_results_with_dry_run(
    results: &[GenerateResult],
    dry_run: bool,
    verb: GenerationVerb,
) -> bool {
    render_generation_results(
        results,
        |result| {
            if dry_run {
                if let Some(output) = &result.output {
                    print!("{}", output);
                } else if result.changed {
                    println!(
                        "{} {} ({} resources)",
                        format!("{} {}", result.name, verb.dry_run_label()).yellow(),
                        ui::Ui::format_duration(result.duration).green(),
                        result.resource_count.to_string().cyan()
                    );
                } else {
                    println!("{} {}", "Unchanged:".dimmed(), result.name.as_str().bold());
                }
            } else if result.changed {
                verb.print_changed(result);
            } else {
                println!("{} {}", "Unchanged:".dimmed(), result.name.as_str().bold());
            }
        },
        |result| {
            ui::Ui::print_generation_error(result.name.as_str(), result.error.as_ref().unwrap())
        },
    )
}
