use bevy::prelude::*;
use es_fluent::{FluentDisplay, ToFluentString};
use std::fmt::Display;

/// A component that holds a value implementing the EsFluent trait.
/// This component automatically updates the associated Text component
/// when the language changes.
#[derive(Clone, Component)]
pub struct FluentText<T: FluentDisplay + Clone> {
    pub value: T,
}

impl<T: FluentDisplay + Clone> FluentText<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: FluentDisplay + Clone> Display for FluentText<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.to_fluent_string())
    }
}