//! Modules whose locale resources are compiled into the binary.

mod assets;
mod module;

pub use assets::{EmbeddedLanguage, EmbeddedResource};
pub use module::EmbeddedI18nModule;

#[cfg(test)]
mod tests;
