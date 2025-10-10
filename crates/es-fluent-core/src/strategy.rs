//! This module provides the `DisplayStrategy` enum, which is used to determine
//! how to display a type.

use crate::options::{r#enum::EnumOpts, r#struct::StructOpts};
use strum::{EnumIter, EnumString, IntoStaticStr};

/// The strategy to use for displaying a type.
#[derive(Clone, Copy, Debug, EnumIter, EnumString, Eq, Hash, IntoStaticStr, PartialEq)]
pub enum DisplayStrategy {
    /// Use `es_fluent::FluentDisplay` on the target enum.
    #[strum(serialize = "fluent")]
    FluentDisplay,
    /// Use `std::fmt::Display` on the target enum.
    #[strum(serialize = "std")]
    StdDisplay,
}

impl From<&EnumOpts> for DisplayStrategy {
    fn from(opts: &EnumOpts) -> Self {
        opts.attr_args().display()
    }
}

impl From<&StructOpts> for DisplayStrategy {
    fn from(opts: &StructOpts) -> Self {
        opts.attr_args().display()
    }
}
