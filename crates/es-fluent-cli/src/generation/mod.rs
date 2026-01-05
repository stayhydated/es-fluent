//! FTL generation functionality.
//!
//! This module handles generating FTL files using a monolithic temporary
//! crate that links all workspace crates via Rust's inventory mechanism.

pub mod cache;
mod generator;
mod temp_crate;
mod templates;

pub use generator::generate_for_crate_monolithic;
pub use temp_crate::{
    prepare_monolithic_temp_crate, run_cargo, run_cargo_with_output, run_monolithic,
};
