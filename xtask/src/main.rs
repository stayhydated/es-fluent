mod generate_lang_names;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development tasks for es-fluent", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate language-name resources and supported_locales.rs from ICU4X data
    GenerateLangNames(generate_lang_names::GenerateLangNamesArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateLangNames(args) => generate_lang_names::run(args),
    }
}
