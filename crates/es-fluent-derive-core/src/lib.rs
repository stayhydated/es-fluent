//! Core library for `es-fluent-derive`.
#![cfg_attr(not(test), deny(clippy::panic, clippy::unwrap_used))]

pub mod attribute;
pub mod context;
pub mod error;
pub mod lowered;
pub mod options;
pub mod semantic;
pub mod validation;

pub use error::{EsFluentError, EsFluentResult};
