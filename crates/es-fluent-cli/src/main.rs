mod discovery;
mod errors;
mod format;
mod generator;
mod mode;
mod sync;
mod temp_crate;
mod templates;
mod tui;
mod types;
mod ui;
mod utils;
mod validate;
mod watcher;

use crate::discovery::{count_ftl_resources, discover_crates};
use crate::errors::CliError;
use crate::format::{FormatArgs, run_format};
use crate::mode::FluentParseMode;
use crate::sync::{SyncArgs, run_sync};
use crate::types::GenerateResult;
use crate::utils::{filter_crates_by_package, partition_by_lib_rs};
use crate::validate::{CheckArgs, run_check};
use clap::{Parser, Subcommand};
use miette::Result as MietteResult;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "es-fluent")]
#[command(about = "CLI for generating FTL files from es-fluent derive macros")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate FTL files once for all crates with i18n.toml
    Generate(CommonArgs),

    /// Watch for changes and regenerate FTL files (TUI mode)
    Watch(CommonArgs),

    /// Clean orphan keys from FTL files
    Clean(CleanArgs),

    /// Format FTL files (sort keys A-Z)
    Format(FormatArgs),

    /// Check FTL files for missing keys and variables
    Check(CheckArgs),

    /// Sync missing translations from fallback to other locales
    Sync(SyncArgs),
}

/// Common arguments shared across commands that work with crates.
#[derive(Parser, Clone)]
struct PathArgs {
    /// Path to the crate or workspace root (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package)
    #[arg(short = 'P', long)]
    package: Option<String>,
}

#[derive(Parser)]
struct CommonArgs {
    #[command(flatten)]
    common: PathArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    mode: FluentParseMode,
}

#[derive(Parser)]
struct CleanArgs {
    #[command(flatten)]
    common: PathArgs,
}

fn main() -> MietteResult<()> {
    // Set up miette for fancy error reporting
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .unicode(true)
                .context_lines(2)
                .tab_width(4)
                .color(true)
                .build(),
        )
    }))
    .ok(); // Ignore if already set

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Generate(args) => run_generate(args),
        Commands::Watch(args) => run_watch(args),
        Commands::Clean(args) => run_clean(args),
        Commands::Format(args) => run_format(args),
        Commands::Check(args) => run_check(args),
        Commands::Sync(args) => run_sync(args),
    };

    result.map_err(miette::Report::new)
}

fn run_generate(args: CommonArgs) -> Result<(), CliError> {
    let path = args.common.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_header();

    let crates = discover_crates(&path)?;
    let crates = filter_crates_by_package(crates, args.common.package.as_ref());

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
        ui::print_generating(&krate.name);
    }

    let results = generate_crates(&valid_crates, &args.mode);
    let has_errors = print_generate_results(&results);

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

fn run_watch(args: CommonArgs) -> Result<(), CliError> {
    let path = args.common.path.unwrap_or_else(|| PathBuf::from("."));

    let crates = discover_crates(&path)?;
    let crates = filter_crates_by_package(crates, args.common.package.as_ref());

    if crates.is_empty() {
        ui::print_header();
        ui::print_discovered(&[]);
        return Ok(());
    }

    watcher::watch_all(&crates, &args.mode).map_err(CliError::from)
}

fn run_clean(args: CleanArgs) -> Result<(), CliError> {
    let path = args.common.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_header();

    let crates = discover_crates(&path)?;
    let crates = filter_crates_by_package(crates, args.common.package.as_ref());

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

/// Generate FTL files for multiple crates in parallel.
fn generate_crates(crates: &[&types::CrateInfo], mode: &FluentParseMode) -> Vec<GenerateResult> {
    crates
        .par_iter()
        .map(|krate| {
            let start = Instant::now();
            let result = generator::generate_for_crate(krate, mode);
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

/// Print generation results and return whether there were any errors.
fn print_generate_results(results: &[GenerateResult]) -> bool {
    let mut has_errors = false;
    for result in results {
        if let Some(ref error) = result.error {
            ui::print_generation_error(&result.name, error);
            has_errors = true;
        } else {
            ui::print_generated(&result.name, result.duration, result.resource_count);
        }
    }
    has_errors
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
