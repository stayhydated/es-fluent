#![doc = include_str!("../README.md")]

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
pub use rust_embed as __rust_embed;

#[doc(hidden)]
pub use es_fluent_manager_core as __manager_core;

#[doc(hidden)]
pub use unic_langid;

#[cfg(feature = "build")]
pub mod build {
    pub use es_fluent_toml::build::*;
}

mod traits;
pub use traits::{EsFluentChoice, FluentLabel, FluentLocalizer, FluentLocalizerExt, FluentMessage};

#[doc(hidden)]
pub mod __private {
    pub use crate::traits::{
        FluentArgumentValue, FluentLocalizerExt, IntoFluentArgumentValue, IntoFluentValue,
        localize_label,
    };
}
