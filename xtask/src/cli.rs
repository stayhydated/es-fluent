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
    /// Build generated workspace artifacts
    Build {
        #[command(subcommand)]
        target: BuildCommand,
    },
    /// Preview generated workspace artifacts
    Preview {
        #[command(subcommand)]
        target: PreviewCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum BuildCommand {
    /// Build the Trunk-hosted Bevy demo into web/public/bevy-demo
    BevyDemo,
    /// Build the Trunk-hosted GPUI demo into web/public/gpui-demo
    GpuiDemo,
    /// Build mdBook documentation to web/public/book
    Book,
    /// Build llms.txt and per-chapter Markdown files from mdBook sources
    LlmsTxt,
    /// Build the Dioxus site into web/dist for GitHub Pages
    Web,
}

#[derive(Debug, Subcommand)]
pub enum PreviewCommand {
    /// Preview the generated static site with its GitHub Pages base path
    Web,
}
