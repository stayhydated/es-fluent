mod discovery;
mod errors;
mod format;
mod generator;
mod mode;
mod sync;
mod templates;
mod tui;
mod types;
mod ui;
mod validate;
mod watcher;

use std::path::Path;

use crate::discovery::{count_ftl_resources, discover_crates};
use crate::format::{FormatArgs, run_format};
use crate::mode::FluentParseMode;
use crate::sync::{SyncArgs, run_sync};
use crate::validate::{CheckArgs, run_check};
use clap::{Parser, Subcommand};
use miette::Result as MietteResult;
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

    /// Clean orphan keys from FTL files
    Clean(CleanArgs),

    /// Format FTL files (sort keys A-Z)
    Format(FormatArgs),

    /// Check FTL files for missing keys and variables
    Check(CheckArgs),

    /// Sync missing translations from fallback to other locales
    Sync(SyncArgs),
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

#[derive(Parser)]
struct CleanArgs {
    /// Path to the crate or workspace root (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package)
    #[arg(short = 'P', long)]
    package: Option<String>,

    /// Clean all locales, not just the fallback language
    #[arg(long)]
    all: bool,
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

    match cli.command {
        Commands::Generate(args) => run_generate(args).map_err(|e| miette::miette!("{}", e)),
        Commands::Watch(args) => run_watch(args).map_err(|e| miette::miette!("{}", e)),
        Commands::Clean(args) => run_clean(args).map_err(|e| miette::miette!("{}", e)),
        Commands::Format(args) => run_format(args).map_err(miette::Report::new),
        Commands::Check(args) => run_check(args).map_err(miette::Report::new),
        Commands::Sync(args) => run_sync(args).map_err(miette::Report::new),
    }
}

fn run_generate(args: CommonArgs) -> anyhow::Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_header();

    // Validate that we can discover crates (this implicitly validates i18n.toml)
    let crates = discover_crates_with_validation(&path)?;

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

    let (valid_crates, skipped_crates): (Vec<_>, Vec<_>) =
        crates.iter().partition(|k| k.has_lib_rs);

    for krate in &skipped_crates {
        ui::print_missing_lib_rs(&krate.name);
    }

    for krate in &valid_crates {
        ui::print_generating(&krate.name);
    }

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

fn run_watch(args: CommonArgs) -> anyhow::Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    let crates = discover_crates_with_validation(&path)?;

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

    watcher::watch_all(&crates, &args.mode)
}

fn run_clean(args: CleanArgs) -> anyhow::Result<()> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    ui::print_header();

    let crates = discover_crates_with_validation(&path)?;

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

    let (valid_crates, skipped_crates): (Vec<_>, Vec<_>) =
        crates.iter().partition(|k| k.has_lib_rs);

    for krate in &skipped_crates {
        ui::print_missing_lib_rs(&krate.name);
    }

    for krate in &valid_crates {
        ui::print_cleaning(&krate.name);
    }

    // When --all is specified, we need to clean all locales
    // For now, the clean mode only affects the fallback locale
    // TODO: Implement --all for clean command to iterate over all locales
    if args.all {
        ui::print_cleaning_all_locales();
    }

    let mode = FluentParseMode::Clean;
    let results: Vec<GenerateResult> = valid_crates
        .par_iter()
        .map(|krate| {
            let start = Instant::now();
            let result = generator::generate_for_crate(krate, &mode);
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

    let mut has_errors = false;
    for result in &results {
        if let Some(ref error) = result.error {
            ui::print_generation_error(&result.name, error);
            has_errors = true;
        } else {
            ui::print_cleaned(&result.name, result.duration, result.resource_count);
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Discover crates with proper validation of i18n.toml files.
/// Returns a miette-compatible error with nice formatting if validation fails.
fn discover_crates_with_validation(path: &Path) -> anyhow::Result<Vec<types::CrateInfo>> {
    discover_crates(path).map_err(|e| {
        // Check if this is a config-related error and provide better messaging
        let error_str = e.to_string();
        if error_str.contains("i18n.toml") {
            anyhow::anyhow!(
                "Configuration error: {}\n\n\
                 Help: Ensure your i18n.toml file is valid. Example:\n\n  \
                 fallback_language = \"en\"\n  \
                 assets_dir = \"i18n\"\n",
                error_str
            )
        } else {
            e
        }
    })
}
