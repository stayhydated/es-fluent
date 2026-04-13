//! FTL generation functionality.
//!
//! This module handles generating FTL files using a monolithic runner
//! crate that links all workspace crates via Rust's inventory mechanism.

pub mod cache;
mod execution;
mod runner;
mod templates;

pub(crate) use execution::MonolithicExecutor;
pub use runner::prepare_monolithic_runner_crate;
