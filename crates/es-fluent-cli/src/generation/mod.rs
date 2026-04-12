//! FTL generation functionality.
//!
//! This module handles generating FTL files using a monolithic runner
//! crate that links all workspace crates via Rust's inventory mechanism.

pub mod cache;
mod execution;
mod runner;
mod templates;

#[cfg(test)]
pub(crate) use execution::read_changed_status;
pub use execution::{
    build_check_request, execute_generation_action_monolithic, execute_request_monolithic,
};
pub use runner::prepare_monolithic_runner_crate;
