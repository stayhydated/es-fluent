mod generator;
mod templates;
mod watcher;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Generate FTL files once
    Generate(CommonArgs),

    /// Watch for changes and regenerate FTL files
    Watch(CommonArgs),
}

#[derive(Parser)]
struct CommonArgs {
    /// Path to the crate (defaults to current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Package name in workspace (if in a workspace)
    #[arg(short = 'P', long)]
    package: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate(CommonArgs { path, package }) => {
            let path = path.unwrap_or_else(|| PathBuf::from("."));
            generator::generate_once(&path, package.as_deref())?;
        },
        Commands::Watch(CommonArgs { path, package }) => {
            let path = path.unwrap_or_else(|| PathBuf::from("."));
            watcher::watch(&path, package.as_deref())?;
        },
    }

    Ok(())
}
