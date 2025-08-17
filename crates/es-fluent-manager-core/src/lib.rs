// In: crates/es-fluent-manager-core/src/lib.rs

//! Core functionality for es-fluent managers.
//!
//! This crate provides the core `FluentManager` struct and related traits that can be used
//! by various manager implementations (Bevy, generic, etc.) without relying on singletons.

pub mod localization;

// Re-export the key struct and traits for easy top-level access.
pub use localization::{FluentManager, Localizer, I18nModule, LocalizationError};