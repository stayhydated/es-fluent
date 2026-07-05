//! FTL file operations shared across CLI commands.
//!
//! This module consolidates common FTL parsing and extraction logic
//! used by the format, check, and sync commands.

mod files;
mod locale;
mod parse;

pub use files::{
    CrateFtlLayout, LoadedFtlFile, discover_and_load_ftl_files,
    discover_crate_ftl_files_in_locale_dir, discover_locale_ftl_files, main_ftl_path,
};
pub use locale::LocaleContext;
pub(crate) use locale::{is_real_locale_directory, locale_named_non_directory_paths};
pub use parse::{
    extract_message_keys, extract_variables_from_message,
    extract_variables_from_value_and_attributes, parse_ftl_file,
};
