//! CLI command implementations.

mod check;
mod clean;
mod common;
mod dry_run;
mod format;
mod generate;
mod sync;
mod watch;

pub use check::{CheckArgs, run_check};
pub use clean::{CleanArgs, run_clean};
pub use common::{
    GenerationVerb, LocaleProcessingArgs, WorkspaceArgs, WorkspaceCrates, parallel_generate,
    render_generation_results, render_generation_results_with_dry_run,
};
pub use dry_run::{DryRunDiff, DryRunSummary};
pub use format::{FormatArgs, run_format};
pub use generate::{GenerateArgs, run_generate};
pub use sync::{SyncArgs, run_sync};
pub use watch::{WatchArgs, run_watch};
