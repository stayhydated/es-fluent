//! Clean command implementation.

use crate::core::{CliError, CrateInfo, FluentParseMode, GenerateResult};
use crate::generation::generate_for_crate;
use crate::utils::{
    count_ftl_resources, discover_crates, filter_crates_by_package, partition_by_lib_rs, ui,
};
use clap::Parser;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

/// Arguments for the clean command.
#[derive(Parser)]
pub struct CleanArgs {
    /// Path to the crate or workspace root (defaults to current directory)
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package)
    #[arg(short = 'P', long)]
    pub package: Option<String>,
}

/// Run the clean command.
pub fn run_clean(args: CleanArgs) -> Result<(), CliError> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_header();

    let crates = discover_crates(&path)?;
    let crates = filter_crates_by_package(crates, args.package.as_ref());

    if crates.is_empty() {
        ui::print_discovered(&[]);
        return Ok(());
    }

    ui::print_discovered(&crates);

    let (valid_crates, skipped_crates) = partition_by_lib_rs(&crates);

    for krate in &skipped_crates {
        ui::print_missing_lib_rs(&krate.name);
    }

    for krate in &valid_crates {
        ui::print_cleaning(&krate.name);
    }

    let results = generate_crates(&valid_crates, &FluentParseMode::Clean);
    let has_errors = print_clean_results(&results);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Generate FTL files for multiple crates in parallel (with clean mode).
fn generate_crates(crates: &[&CrateInfo], mode: &FluentParseMode) -> Vec<GenerateResult> {
    crates
        .par_iter()
        .map(|krate| {
            let start = Instant::now();
            let result = generate_for_crate(krate, mode);
            let duration = start.elapsed();
            let resource_count = result
                .as_ref()
                .ok()
                .map(|_| count_ftl_resources(&krate.ftl_output_dir, &krate.name))
                .unwrap_or(0);

            match result {
                Ok(()) => GenerateResult::success(krate.name.clone(), duration, resource_count),
                Err(e) => GenerateResult::failure(krate.name.clone(), duration, e.to_string()),
            }
        })
        .collect()
}

/// Print clean results and return whether there were any errors.
fn print_clean_results(results: &[GenerateResult]) -> bool {
    let mut has_errors = false;
    for result in results {
        if let Some(ref error) = result.error {
            ui::print_generation_error(&result.name, error);
            has_errors = true;
        } else {
            ui::print_cleaned(&result.name, result.duration, result.resource_count);
        }
    }
    has_errors
}
