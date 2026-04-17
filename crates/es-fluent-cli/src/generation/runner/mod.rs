//! Shared functionality for creating and running the runner crate.
//!
//! The CLI uses a monolithic runner crate at workspace root that links all workspace
//! crates to access their inventory registrations through a single binary.

use anyhow::{Result, anyhow};
use camino::Utf8Path;
use std::path::Path;

mod config;
mod exec;
mod monolithic;

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn utf8_path_string(path: &Path, context: &str) -> Result<String> {
    Utf8Path::from_path(path)
        .map(|path| path.as_str().to_string())
        .ok_or_else(|| anyhow!("{context} must be valid UTF-8: {}", path.display()))
}

#[cfg(test)]
pub use exec::{run_cargo, run_cargo_with_output};
pub use monolithic::{prepare_monolithic_runner_crate, run_monolithic};

#[cfg(test)]
mod tests;
