//! Shared functionality for creating and running the runner crate.
//!
//! The CLI uses a monolithic runner crate at workspace root that links all workspace
//! crates to access their inventory registrations through a single binary.

use anyhow::{Result, anyhow};
use camino::Utf8Path;
use std::path::Path;

mod config;
mod dep_info;
mod exec;
mod monolithic;
mod profiles;

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn utf8_path_string(path: &Path, context: &str) -> Result<String> {
    Utf8Path::from_path(path)
        .map(|path| path.as_str().to_string())
        .ok_or_else(|| anyhow!("{context} must be valid UTF-8: {}", path.display()))
}

/// Writes `contents` to `path` only when the file content differs.
///
/// Unchanged runner sources keep their modification time, so Cargo does not
/// relink the runner and dep-info staleness checks stay meaningful.
fn write_if_changed(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let contents = contents.as_ref();
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    fs_err::write(path, contents)
}

#[cfg(test)]
pub use exec::{run_cargo, run_cargo_with_output};
pub use monolithic::{
    acquire_monolithic_runner_lock, prepare_monolithic_runner_crate, run_monolithic,
};

#[cfg(test)]
mod tests;
