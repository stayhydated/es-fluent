//! Utility functions shared across CLI commands.

mod discovery;
mod helpers;
pub(crate) mod paths;
pub mod ui;

pub use discovery::count_ftl_resources;
pub(crate) use discovery::{
    DiscoveryScope, discover_i18n_package_names, discover_workspace_scoped,
};
pub use helpers::{filter_crates_by_package, partition_by_lib_rs};
