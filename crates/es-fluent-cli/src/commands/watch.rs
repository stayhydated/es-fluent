//! Watch command implementation.

use crate::commands::{WorkspaceArgs, WorkspaceCrates};
use crate::core::{CliError, FluentParseMode};
use crate::tui::watch_all;
use crate::utils::ui;
use clap::Parser;

/// Arguments for the watch command.
#[derive(Parser)]
pub struct WatchArgs {
    #[command(flatten)]
    pub workspace: WorkspaceArgs,

    /// Parse mode for FTL generation
    #[arg(short, long, value_enum, default_value_t = FluentParseMode::default())]
    pub mode: FluentParseMode,
}

/// Run the watch command.
pub fn run_watch(args: WatchArgs) -> Result<(), CliError> {
    let workspace = WorkspaceCrates::discover(args.workspace)?;

    if !workspace.print_discovery(ui::print_header) {
        return Ok(());
    }

    watch_all(&workspace.crates, &workspace.workspace_info, &args.mode).map_err(CliError::from)
}
