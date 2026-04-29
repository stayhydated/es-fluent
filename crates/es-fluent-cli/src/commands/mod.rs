//! CLI command implementations.

mod check;
mod clean;
mod common;
mod dry_run;
mod format;
mod generate;
mod init;
mod sync;
mod tree;
mod watch;

pub(crate) use check::{CheckArgs, run_check};
pub(crate) use clean::{CleanArgs, run_clean};
#[cfg(test)]
pub(crate) use common::WorkspaceArgs;
pub(crate) use format::{FormatArgs, run_format};
pub(crate) use generate::{GenerateArgs, run_generate};
pub(crate) use init::{InitArgs, run_init};
pub(crate) use sync::{SyncArgs, run_sync};
pub(crate) use tree::{TreeArgs, run_tree};
pub(crate) use watch::{WatchArgs, run_watch};
