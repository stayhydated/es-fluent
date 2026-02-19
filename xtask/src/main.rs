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

        /// Discover locales from CLDR data instead of reading existing files
        #[arg(long)]
        discover: bool,

        /// CLDR tag to download (e.g., "48.1.0"). Requires --discover.
        #[arg(long, requires = "discover")]
        cldr_tag: Option<String>,

        /// Path to local CLDR archive (directory or ZIP). Requires --discover.
        #[arg(long, requires = "discover")]
        cldr_path: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenerateLangNames {
            ftl_output,
            rs_output,
            i18n_dir,
            discover,
            cldr_tag,
            cldr_path,
        } => {
            generate_lang_names::run(
                ftl_output, rs_output, i18n_dir, discover, cldr_tag, cldr_path,
            )?;
        },
    }

    Ok(())
}
