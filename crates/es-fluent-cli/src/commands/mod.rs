//! CLI command implementations.

mod check;
mod clean;
mod format;
mod generate;
mod sync;
mod watch;

pub use check::{CheckArgs, run_check};
pub use clean::{CleanArgs, run_clean};
pub use format::{FormatArgs, run_format};
pub use generate::{GenerateArgs, run_generate};
pub use sync::{SyncArgs, run_sync};
pub use watch::{WatchArgs, run_watch};
