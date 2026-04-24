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
    /// Build the Trunk-hosted Bevy demo into web/public/bevy-demo
    BuildBevyDemo,
    /// Build mdBook documentation to web/public/book
    BuildBook,
    /// Build llms.txt from mdBook sources to web/public/llms.txt
    BuildLlmsTxt,
    /// Build the Dioxus site into web/dist for GitHub Pages
    BuildWeb,
}
