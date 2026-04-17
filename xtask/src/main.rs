mod cli;
mod commands;
mod util;
mod wasm_examples;

use clap::Parser;

use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::BuildBook => commands::build_book::run(),
        Command::BuildLlmsTxt => commands::build_llms_txt::run(),
        Command::BuildWasmExamples => commands::build_wasm_examples::run(),
        Command::GenerateWasmExamplesSchema => commands::generate_wasm_examples_schema::run(),
        Command::VerifyWasmExamples => commands::verify_wasm_examples::run(),
    }
}
