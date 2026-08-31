//! Sync command for synchronizing missing translations across locales.
//!
//! This module provides functionality to sync missing translation keys
//! from the fallback language to other locales, preserving existing translations.

mod execution;
mod preflight;
mod report;
mod selection;

mod locale;
mod merge;

pub use execution::run_sync;
pub(crate) use execution::run_sync_with_text_mode;
pub(crate) use locale::sync_crate;
pub(crate) use report::SyncTextMode;
pub use selection::SyncArgs;

#[cfg(test)]
mod tests;
