//! Utility functions shared across CLI commands.

mod discovery;
mod helpers;
pub mod ui;

pub use discovery::{count_ftl_resources, discover_workspace};
pub use helpers::{filter_crates_by_package, partition_by_lib_rs};
