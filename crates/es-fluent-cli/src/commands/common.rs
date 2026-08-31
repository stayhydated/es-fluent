mod command;
mod generation;
mod output;
mod paths;
mod workspace;

pub use command::run_generation_command;
pub use generation::run_generation_for_crates;
pub(crate) use generation::run_generation_for_crates_with_transaction;
#[cfg(test)]
pub use output::render_generation_results;
pub use output::{GenerationVerb, OutputFormat, render_generation_results_with_dry_run};
pub(crate) use paths::validate_generation_paths;
pub use workspace::{WorkspaceArgs, WorkspaceCrates};

#[cfg(test)]
use crate::core::{CliError, GenerateResult};

#[cfg(test)]
mod tests;
