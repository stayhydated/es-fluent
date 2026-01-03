//! FTL generation functionality.
//!
//! This module handles creating temporary crates and generating FTL files
//! using Rust's inventory mechanism.

mod generator;
mod temp_crate;
mod templates;

pub use generator::generate_for_crate;
pub use temp_crate::{
    prepare_temp_crate, run_cargo, run_cargo_with_output,
};
pub use templates::{CargoTomlTemplate, CheckRsTemplate, GitignoreTemplate, MainRsTemplate};
