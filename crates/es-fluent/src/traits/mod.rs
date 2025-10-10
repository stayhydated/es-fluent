//! This module contains traits for converting types into Fluent messages.

pub mod fluent_choice;
pub mod fluent_display;

pub use fluent_choice::EsFluentChoice;
pub use fluent_display::{FluentDisplay, ToFluentString};
