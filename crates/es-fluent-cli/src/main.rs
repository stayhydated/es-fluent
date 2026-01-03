use clap::{Parser, Subcommand};
use es_fluent_cli::commands::{
    CheckArgs, CleanArgs, FormatArgs, GenerateArgs, SyncArgs, WatchArgs, run_check, run_clean,
    run_format, run_generate, run_sync, run_watch,
};
use miette::Result as MietteResult;

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
    Generate(GenerateArgs),

    /// Watch for changes and regenerate FTL files (TUI mode)
    Watch(WatchArgs),

    /// Clean orphan keys from FTL files
    Clean(CleanArgs),

    /// Format FTL files (sort keys A-Z)
    Format(FormatArgs),

    /// Check FTL files for missing keys and variables
    Check(CheckArgs),

    /// Sync missing translations from fallback to other locales
    Sync(SyncArgs),
}

fn main() -> MietteResult<()> {
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
    .ok();

    let cli = Cli::parse();
    
    // Initialize logging
    es_fluent_cli::utils::ui::init_logging();

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
