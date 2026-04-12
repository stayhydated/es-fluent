//! FTL generation functionality.
//!
//! This module handles generating FTL files using a monolithic runner
//! crate that links all workspace crates via Rust's inventory mechanism.

pub mod cache;
mod generator;
mod runner;
mod templates;

pub use generator::generate_for_crate_monolithic;
pub use runner::{prepare_monolithic_runner_crate, run_monolithic};
