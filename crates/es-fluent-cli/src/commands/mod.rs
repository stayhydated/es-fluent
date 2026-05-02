//! CLI command implementations.

mod add_locale;
mod check;
mod clean;
mod common;
mod doctor;
mod dry_run;
mod format;
mod generate;
mod init;
mod status;
mod sync;
mod tree;
mod watch;

pub(crate) use add_locale::{AddLocaleArgs, run_add_locale};
pub(crate) use check::{CheckArgs, run_check};
pub(crate) use clean::{CleanArgs, run_clean};
#[cfg(test)]
pub(crate) use common::{OutputFormat, WorkspaceArgs};
pub(crate) use doctor::{DoctorArgs, run_doctor};
pub(crate) use format::{FormatArgs, run_format};
pub(crate) use generate::{GenerateArgs, run_generate};
#[cfg(test)]
pub(crate) use init::InitManager;
pub(crate) use init::{InitArgs, run_init};
pub(crate) use status::{StatusArgs, run_status};
pub(crate) use sync::{SyncArgs, run_sync};
pub(crate) use tree::{TreeArgs, run_tree};
pub(crate) use watch::{WatchArgs, run_watch};
