use clap::{Args, Parser, Subcommand};

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
    /// Release workspace crates in registry dependency order
    Release {
        #[command(subcommand)]
        action: ReleaseCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum BuildCommand {
    /// Build the Trunk-hosted Bevy demo into web/public/bevy-demo
    BevyDemo,
    /// Build mdBook documentation to web/public/book
    Book,
    /// Build llms.txt and per-chapter Markdown files from mdBook sources
    LlmsTxt,
    /// Build the Dioxus site into web/dist for GitHub Pages
    Web,
}

#[derive(Debug, Subcommand)]
pub enum ReleaseCommand {
    /// Print the publish order for workspace crates
    Plan,
    /// Publish workspace crates one at a time in dependency order
    Publish(ReleasePublishArgs),
}

#[derive(Args, Debug)]
pub struct ReleasePublishArgs {
    /// Upload packages to the registry. Without this flag, only commands are printed.
    #[arg(long)]
    pub execute: bool,

    /// Resume publishing at this package name.
    #[arg(long)]
    pub from: Option<String>,

    /// Registry name to pass through to cargo publish.
    #[arg(long)]
    pub registry: Option<String>,

    /// Allow dirty working directories when packaging.
    #[arg(long)]
    pub allow_dirty: bool,

    /// Skip cargo's package verification build.
    #[arg(long)]
    pub no_verify: bool,

    /// Use plain cargo publish, including dev-dependencies.
    #[arg(long)]
    pub include_dev_deps: bool,

    /// Treat an already-uploaded package version as success.
    #[arg(long)]
    pub skip_existing: bool,

    /// Retry a failed cargo publish command this many times.
    #[arg(long, default_value_t = 3)]
    pub retries: u32,

    /// Seconds to wait before retrying a failed publish command.
    #[arg(long, default_value_t = 20)]
    pub retry_delay_seconds: u64,
}
