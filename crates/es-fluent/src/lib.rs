#![doc = include_str!("../README.md")]
#![allow(clippy::needless_doctest_main)]

#[doc(hidden)]
pub mod meta;

#[doc(hidden)]
pub mod registry;

#[cfg(feature = "derive")]
pub use es_fluent_derive::{EsFluent, EsFluentChoice, EsFluentLabel, EsFluentVariants};

#[doc(hidden)]
pub use fluent_bundle::FluentValue;

#[doc(hidden)]
pub use inventory as __inventory;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid;

mod traits;
pub use traits::{
    EsFluentChoice, FluentArgs, FluentLabel, FluentLocalizer, FluentLocalizerExt,
    FluentLocalizerLookup, FluentMessage, FluentMessageLookup,
};

#[doc(hidden)]
pub mod __private {
    pub use crate::traits::{
        FluentArgumentValue, FluentBorrowedArgumentValue, FluentLocalizerExt,
        FluentOptionalArgumentValue, IntoFluentArgumentValue, IntoFluentValue, localize_label,
    };
}
