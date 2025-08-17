// In: crates/es-fluent-manager-generic/src/lib.rs

//! A generic manager for es-fluent.
//!
//! This crate provides a `FluentManager` struct that can be used to manage
//! localization in contexts where a struct-based holder is preferred over
//! a static singleton.
//!
//! # Example
//!
//! ```rust
//! use es_fluent_manager_generic::FluentManager;
//! use unic_langid::LanguageIdentifier;
//!
//! fn main() {
//!     let mut manager = FluentManager::new_with_discovered_modules();
//!     
//!     let lang = "en-US".parse::<LanguageIdentifier>().expect("Invalid language identifier");
//!     manager.select_language(&lang);
//!     
//!     let localized = manager.localize("hello-world", None);
//!     println!("Localized message: {:?}", localized);
//! }
//! ```

// Re-export the key struct and its methods for easy top-level access.
pub use es_fluent_manager_core::FluentManager;