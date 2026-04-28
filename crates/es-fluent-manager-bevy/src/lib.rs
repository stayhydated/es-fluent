#![doc = include_str!("../README.md")]

pub use bevy;
pub use inventory;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use inventory as __inventory;

#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::BevyFluentText;
#[cfg(feature = "macros")]
pub use es_fluent_manager_macros::define_bevy_i18n_module as define_i18n_module;

pub use unic_langid;

#[doc(hidden)]
pub use unic_langid as __unic_langid;

mod assets;
mod context;
mod locale;
mod registration;

pub mod components;
pub mod plugin;
pub mod systems;

pub use assets::*;
pub use components::*;
pub use context::*;
pub use es_fluent::FluentMessage;
pub use locale::*;
pub use plugin::*;
pub use registration::*;
pub use systems::*;

pub(crate) use assets::BundleBuildFailures;
pub(crate) use locale::{LanguageSelection, PendingLanguageChange};

#[doc(hidden)]
pub use unic_langid::langid as __langid;

#[cfg(test)]
mod tests;
