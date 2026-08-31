mod arguments;
mod localizer;
mod wrappers;

pub use arguments::{FluentArgs, FluentLocalizerLookup, FluentMessageLookup};
pub use localizer::{FluentLocalizer, FluentLocalizerExt, FluentMessage};
pub use wrappers::{
    FluentArgumentValue, FluentBorrowedArgumentValue, FluentOptionalArgumentValue,
    IntoFluentArgumentValue, IntoFluentValue,
};

#[cfg(test)]
use crate::{
    FluentValue,
    registry::{StaticFluentMessageKey, StaticFluentVariantKey},
};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
mod tests;
