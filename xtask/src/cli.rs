use crate::commands::generate_lang_names::GenerateLangNamesArgs;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    about = "Workspace maintenance tasks.",
    disable_help_subcommand = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate language-name resources and supported_locales.rs from ICU4X data
    GenerateLangNames(GenerateLangNamesArgs),
    /// Build mdBook documentation to web/public/book
    BuildBook,
    /// Build llms.txt from mdBook sources to web/public/llms.txt
    BuildLlmsTxt,
}
