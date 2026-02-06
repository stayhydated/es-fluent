use clap::{Parser, Subcommand};
use es_fluent_cli::commands::{
    CheckArgs, CleanArgs, FormatArgs, GenerateArgs, SyncArgs, WatchArgs, run_check, run_clean,
    run_format, run_generate, run_sync, run_watch,
};
use miette::Result as MietteResult;

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: CargoCommand,
}

#[derive(Subcommand)]
enum CargoCommand {
    /// CLI for generating FTL files from es-fluent derive macros
    #[command(name = "es-fluent", version)]
    EsFluent {
        #[command(subcommand)]
        command: Commands,

        /// Enable E2E testing mode
        #[arg(long, global = true, hide = true)]
        e2e: bool,
    },
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
    Fmt(FormatArgs),

    /// Check FTL files for missing keys and variables
    Check(CheckArgs),

    /// Sync missing translations from fallback to other locales
    Sync(SyncArgs),
}

fn main() -> MietteResult<()> {
    // Parse first to check for e2e flag before setting up miette/logging
    let cli = Cli::parse();
    let CargoCommand::EsFluent { command, e2e } = cli.command;

    if e2e {
        es_fluent_cli::utils::ui::set_e2e_mode(true);
    }

    let no_color = std::env::var("NO_COLOR").is_ok();
    let use_color = !es_fluent_cli::utils::ui::is_e2e() && !no_color;
    let use_links = es_fluent_cli::utils::ui::terminal_links_enabled();

    miette::set_hook(Box::new(move |_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(use_links)
                .unicode(true)
                .context_lines(2)
                .tab_width(4)
                .color(use_color)
                .build(),
        )
    }))
    .ok();

    let result = match command {
        Commands::Generate(args) => run_generate(args),
        Commands::Watch(args) => run_watch(args),
        Commands::Clean(args) => run_clean(args),
        Commands::Fmt(args) => run_format(args),
        Commands::Check(args) => run_check(args),
        Commands::Sync(args) => run_sync(args),
    };

    result.map_err(miette::Report::new)
}
