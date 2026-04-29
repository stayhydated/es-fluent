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

pub use check::{CheckArgs, run_check};
pub use clean::{CleanArgs, run_clean};
pub use common::WorkspaceArgs;
pub use dry_run::{DryRunDiff, DryRunSummary};
pub use format::{FormatArgs, run_format};
pub use generate::{GenerateArgs, run_generate};
pub use init::{InitArgs, InitManager, run_init};
pub use sync::{SyncArgs, run_sync};
pub use tree::{TreeArgs, run_tree};
pub use watch::{WatchArgs, run_watch};
