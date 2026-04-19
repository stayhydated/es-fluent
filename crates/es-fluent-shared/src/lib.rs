//! Shared runtime-safe types and helpers for the es-fluent ecosystem.

pub mod error;
pub mod language;
pub mod meta;
pub mod namer;
pub mod namespace;
mod namespace_resolver;
pub mod path_utils;
pub mod registry;

pub use error::{EsFluentError, EsFluentResult};
pub use language::{CanonicalLanguageIdentifierError, parse_canonical_language_identifier};
pub use path_utils::{parse_language_entry, validate_assets_dir};
