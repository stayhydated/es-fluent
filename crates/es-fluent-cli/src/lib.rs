#![doc = include_str!("../README.md")]

mod commands;
mod core;
mod ftl;
mod generation;
mod tui;
mod utils;

pub use commands::{
    CheckArgs, CleanArgs, DryRunDiff, DryRunSummary, FormatArgs, GenerateArgs, SyncArgs, TreeArgs,
    WatchArgs, WorkspaceArgs, run_check, run_clean, run_format, run_generate, run_sync, run_tree,
    run_watch,
};
pub use core::{CliError, FluentParseMode};
pub use utils::ui::{is_e2e, set_e2e_mode, terminal_links_enabled};

#[cfg(test)]
pub(crate) mod test_fixtures;
