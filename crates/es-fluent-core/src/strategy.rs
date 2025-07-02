use crate::options::{r#enum::EnumOpts, r#struct::StructOpts};
use strum::{EnumIter, EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, EnumIter, EnumString, Eq, Hash, IntoStaticStr, PartialEq)]
pub enum DisplayStrategy {
    /// use es_fluent::FluentDisplay on the target enum
    #[strum(serialize = "fluent")]
    FluentDisplay,
    /// use std::fmt::Display on the target enum
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
