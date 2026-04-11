//! Compatibility re-exports for shared FTL file utilities.

pub use crate::ftl::{
    CrateFtlLayout, FtlFileInfo, LoadedFtlFile, discover_and_load_ftl_files,
    discover_crate_ftl_files_in_locale_dir, discover_ftl_files, discover_locale_ftl_files,
    discover_nested_ftl_files, load_ftl_files, locale_output_dir, main_ftl_path,
    parse_ftl_file_with_errors,
};
