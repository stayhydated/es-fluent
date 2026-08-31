//! Caching utilities for CLI performance optimization.
//!
//! This module provides caching for expensive operations like:
//! - Cargo metadata results
//! - Runner binary staleness detection via content hashing

mod cargo;
mod inputs;
mod metadata;
mod runner;

pub(crate) use cargo::{cargo_inputs, configured_cargo_home};
pub use inputs::{compute_crate_inputs_hash, compute_workspace_inputs_hash};
pub use metadata::MetadataCache;
pub use runner::RunnerCache;

#[cfg(test)]
pub(crate) use inputs::compute_workspace_inputs_hash_with_cargo_home;
#[cfg(test)]
use inputs::{hash_optional_file, hash_rs_sources};

#[cfg(test)]
mod tests;
