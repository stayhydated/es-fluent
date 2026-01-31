//! Core library for `es-fluent-derive`.

pub mod error;
pub mod meta;
pub mod namer;
pub mod options;
pub mod path_utils;
pub mod registry;
pub mod validation;

// Re-export common types for easier access across the ecosystem
pub use error::{EsFluentError, EsFluentResult};
pub use path_utils::{
    create_metadata_dir, get_all_locales, get_es_fluent_temp_dir, get_metadata_inventory_path,
    get_metadata_result_path, parse_language_entry, validate_assets_dir, write_metadata_result,
};
