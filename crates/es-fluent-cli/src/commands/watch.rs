//! Watch command implementation.

use crate::core::{CliError, FluentParseMode};
use crate::tui::watch_all;
use crate::utils::{discover_crates, filter_crates_by_package, ui};
use clap::Parser;
use std::path::PathBuf;

/// Arguments for the watch command.
#[derive(Parser)]
pub struct WatchArgs {
    /// Path to the crate or workspace root (defaults to current directory)
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Package name to filter (if in a workspace, only process this package)
    #[arg(short = 'P', long)]
    pub package: Option<String>,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,
}

/// Run the watch command.
pub fn run_watch(args: WatchArgs) -> Result<(), CliError> {
    let path = args.path.unwrap_or_else(|| PathBuf::from("."));

    let crates = discover_crates(&path)?;
    let crates = filter_crates_by_package(crates, args.package.as_ref());

    if crates.is_empty() {
        ui::print_header();
        ui::print_discovered(&[]);
        return Ok(());
    }

    watch_all(&crates, &args.mode).map_err(CliError::from)
}
