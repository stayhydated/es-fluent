use super::FluentParseMode;

/// Command line arguments for the generator.
#[derive(clap::Parser)]
pub struct GeneratorArgs {
    #[command(subcommand)]
    pub(super) action: Action,
}

#[derive(clap::Subcommand)]
pub(super) enum Action {
    /// Generate FTL files
    Generate {
        /// Parse mode
        #[arg(long, default_value_t = FluentParseMode::default())]
        mode: FluentParseMode,
        /// Dry run (don't write changes)
        #[arg(long)]
        dry_run: bool,
    },
    /// Clean FTL files (remove orphans)
    Clean {
        /// Clean all locales
        #[arg(long)]
        all: bool,
        /// Dry run (don't write changes)
        #[arg(long)]
        dry_run: bool,
    },
}
