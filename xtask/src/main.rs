mod cli;
mod commands;
mod util;
use clap::Parser;

use cli::{BuildCommand, Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { target } => match target {
            BuildCommand::BevyDemo => commands::build_bevy_demo::run(),
            BuildCommand::Book => commands::build_book::run(),
            BuildCommand::LlmsTxt => commands::build_llms_txt::run(),
            BuildCommand::Web => commands::build_web::run(),
        },
    }
}
