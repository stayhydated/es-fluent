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
    /// Build mdBook documentation to web/public/book
    BuildBook,
    /// Build llms.txt from mdBook sources to web/public/llms.txt
    BuildLlmsTxt,
    /// Build declared wasm examples from the central manifest
    BuildWasmExamples,
    /// Generate the JSON schema for the wasm example manifest
    GenerateWasmExamplesSchema,
    /// Verify built wasm examples include their required markers
    VerifyWasmExamples,
}
