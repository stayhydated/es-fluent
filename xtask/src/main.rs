mod cli;
mod commands;
mod util;
use clap::Parser;

use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::BuildBevyDemo => commands::build_bevy_demo::run(),
        Command::BuildBook => commands::build_book::run(),
        Command::BuildLlmsTxt => commands::build_llms_txt::run(),
    }
}
