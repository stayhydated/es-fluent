//! FTL generation functionality.
//!
//! This module handles creating temporary crates and generating FTL files
//! using Rust's inventory mechanism.

mod generator;
mod temp_crate;
mod templates;

pub use generator::generate_for_crate;
pub use temp_crate::{
    create_temp_dir, get_es_fluent_dep, run_cargo, run_cargo_with_output, write_cargo_toml,
    write_main_rs,
};
pub use templates::{CargoTomlTemplate, CheckRsTemplate, GitignoreTemplate, MainRsTemplate};
