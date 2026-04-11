//! Utility functions shared across CLI commands.

mod discovery;
pub mod ftl;
mod helpers;
pub mod ui;

pub use discovery::{count_ftl_resources, discover_crates, discover_workspace};
pub use ftl::{
    CrateFtlLayout, FtlFileInfo, LoadedFtlFile, discover_and_load_ftl_files,
    discover_crate_ftl_files_in_locale_dir, discover_ftl_files, discover_locale_ftl_files,
    discover_nested_ftl_files, load_ftl_files, locale_output_dir, main_ftl_path,
    parse_ftl_file_with_errors,
};
pub use helpers::{filter_crates_by_package, get_all_locales, partition_by_lib_rs};
