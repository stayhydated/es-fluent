// In: crates/es-fluent/src/lib.rs

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice};

pub use fluent_bundle::FluentValue;

mod traits;
pub use traits::*;

pub mod localization;

// Re-export the key functions for easy top-level access.
pub use localization::{set_context, with_context, localize};