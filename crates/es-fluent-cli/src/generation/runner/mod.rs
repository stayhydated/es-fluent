//! Shared functionality for creating and running the runner crate.
//!
//! The CLI uses a monolithic runner crate at workspace root that links all workspace
//! crates to access their inventory registrations through a single binary.

mod config;
mod exec;
mod monolithic;

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
pub use exec::{run_cargo, run_cargo_with_output};
pub use monolithic::{prepare_monolithic_runner_crate, run_monolithic};

#[cfg(test)]
mod tests;
