mod discovery;
mod generator;
mod mode;
mod templates;
mod tui;
mod types;
mod ui;
mod watcher;

use crate::discovery::{count_ftl_resources, discover_crates};
use crate::mode::FluentParseMode;
use anyhow::Result;
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Result of generating FTL for a single crate.
struct GenerateResult {
    name: String,
    duration: Duration,
    resource_count: usize,
    error: Option<String>,
}

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
}

#[derive(Parser)]
struct CommonArgs {
    /// Path to the crate or workspace root (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package)
    #[arg(short = 'P', long)]
    package: Option<String>,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    mode: FluentParseMode,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate(args) => run_generate(args),
        Commands::Watch(args) => run_watch(args),
    }
}

fn run_generate(args: CommonArgs) -> Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_header();

    // Discover all crates with i18n.toml
    let crates = discover_crates(&path)?;

    // Filter by package if specified
    let crates: Vec<_> = if let Some(ref pkg) = args.package {
        crates.into_iter().filter(|c| &c.name == pkg).collect()
    } else {
        crates
    };

    if crates.is_empty() {
        ui::print_discovered(&[]);
        return Ok(());
    }

    ui::print_discovered(&crates);

    // Separate crates with and without lib.rs
    let (valid_crates, skipped_crates): (Vec<_>, Vec<_>) =
        crates.iter().partition(|k| k.has_lib_rs);

    // Print skipped crates first
    for krate in &skipped_crates {
        ui::print_missing_lib_rs(&krate.name);
    }

    // Print that we're generating for all valid crates
    for krate in &valid_crates {
        ui::print_generating(&krate.name);
    }

    // Generate in parallel and collect results
    let mode = &args.mode;
    let results: Vec<GenerateResult> = valid_crates
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

            GenerateResult {
                name: krate.name.clone(),
                duration,
                resource_count,
                error: result.err().map(|e| e.to_string()),
            }
        })
        .collect();

    // Print results in order
    let mut has_errors = false;
    for result in &results {
        if let Some(ref error) = result.error {
            ui::print_generation_error(&result.name, error);
            has_errors = true;
        } else {
            ui::print_generated(&result.name, result.duration, result.resource_count);
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

fn run_watch(args: CommonArgs) -> Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    // Discover all crates with i18n.toml
    let crates = discover_crates(&path)?;

    // Filter by package if specified
    let crates: Vec<_> = if let Some(ref pkg) = args.package {
        crates.into_iter().filter(|c| &c.name == pkg).collect()
    } else {
        crates
    };

    if crates.is_empty() {
        ui::print_header();
        ui::print_discovered(&[]);
        return Ok(());
    }

    // Run the TUI watcher
    watcher::watch_all(&crates, &args.mode)
}
