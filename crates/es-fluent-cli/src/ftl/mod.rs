//! FTL file operations shared across CLI commands.
//!
//! This module consolidates common FTL parsing and extraction logic
//! used by the format, check, and sync commands.

mod locale;
mod parse;

pub use locale::{LocaleContext, collect_all_available_locales};
pub use parse::{
    extract_message_keys, extract_variables_from_message, extract_variables_from_pattern,
    parse_ftl_file,
};
