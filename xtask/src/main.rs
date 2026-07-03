mod cli;
mod commands;
use clap::Parser as _;

use cli::{BuildCommand, CheckCommand, Cli, Command, ReleaseCommand};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { target } => match target {
            BuildCommand::BevyDemo => commands::build_bevy_demo::run(),
            BuildCommand::GpuiDemo => commands::build_gpui_demo::run(),
            BuildCommand::Book => commands::build_book::run(),
            BuildCommand::LlmsTxt => commands::build_llms_txt::run(),
            BuildCommand::Web => commands::build_web::run(),
        },
        Command::Check { target } => match target {
            CheckCommand::FtlOwnership => commands::ftl_ownership::run(),
        },
        Command::Release { action } => match action {
            ReleaseCommand::Plan => commands::release::plan(),
            ReleaseCommand::Publish(args) => commands::release::publish(&args),
        },
    }
}
