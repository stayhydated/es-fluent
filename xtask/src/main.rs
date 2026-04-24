mod cli;
mod commands;
mod util;
use clap::Parser;

use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::BevyDemo => commands::build_bevy_demo::run(),
        Command::Book => commands::build_book::run(),
        Command::LlmsTxt => commands::build_llms_txt::run(),
        Command::Web => commands::build_web::run(),
    }
}
