//! Utility functions shared across CLI commands.

mod discovery;
pub mod ftl;
mod helpers;
pub mod ui;

pub use discovery::{count_ftl_resources, discover_crates, discover_workspace};
pub use ftl::{
    FtlFileInfo, LoadedFtlFile, discover_and_load_ftl_files, discover_ftl_files, load_ftl_files,
};
pub use helpers::{filter_crates_by_package, get_all_locales, partition_by_lib_rs};
