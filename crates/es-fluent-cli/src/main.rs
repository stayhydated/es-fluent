mod discovery;
mod generator;
mod templates;
mod types;
mod ui;
mod watcher;

use crate::discovery::{count_ftl_resources, discover_crates};
use crate::types::CrateState;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

    /// Watch for changes and regenerate FTL files
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

    // Initialize states
    let mut states: HashMap<String, CrateState> = HashMap::new();

    // Process each crate
    for krate in &crates {
        if !krate.has_lib_rs {
            states.insert(krate.name.clone(), CrateState::MissingLibRs);
            continue;
        }

        states.insert(krate.name.clone(), CrateState::Generating);
        ui::print_generating(&krate.name);

        let start = Instant::now();
        match generator::generate_for_crate(krate) {
            Ok(()) => {
                let duration = start.elapsed();
                let resource_count = count_ftl_resources(&krate.ftl_output_dir, &krate.name);
                ui::print_generated(&krate.name, duration, resource_count);
                states.insert(krate.name.clone(), CrateState::Watching { resource_count });
            },
            Err(e) => {
                ui::print_generation_error(&krate.name, &e.to_string());
                states.insert(
                    krate.name.clone(),
                    CrateState::Error {
                        message: e.to_string(),
                    },
                );
            },
        }
    }

    // Print final summary
    ui::print_summary(&crates, &states);

    // Check if any crates had generation errors (not counting missing lib.rs)
    let has_errors = states.values().any(|s| s.is_generation_error());
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

fn run_watch(args: CommonArgs) -> Result<()> {
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

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        ui::print_shutdown();
        r.store(false, Ordering::SeqCst);
        std::process::exit(0);
    })?;

    // Run the watcher
    watcher::watch_all(&crates, running)
}
