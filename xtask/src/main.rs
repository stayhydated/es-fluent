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
    GenerateLangNames {
        /// Optional path to es-fluent-lang.ftl (defaults to crate location)
        #[arg(long)]
        ftl_output: Option<String>,

        /// Optional path to supported_locales.rs (defaults to crate location)
        #[arg(long)]
        rs_output: Option<String>,

        /// Optional path to i18n directory (defaults to crate location)
        #[arg(long)]
        i18n_dir: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateLangNames {
            ftl_output,
            rs_output,
            i18n_dir,
        } => {
            generate_lang_names::run(ftl_output, rs_output, i18n_dir)?;
        },
    }

    Ok(())
}
