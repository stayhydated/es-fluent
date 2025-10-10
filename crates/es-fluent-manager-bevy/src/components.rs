use bevy::prelude::*;
use es_fluent::ToFluentString;
use std::fmt::Display;

/// A component that holds a value that can be localized.
/// This component automatically updates the associated Text component
/// when the language changes or its value changes.
#[derive(Clone, Component)]
pub struct FluentText<T: ToFluentString + Clone> {
    pub value: T,
}

impl<T: ToFluentString + Clone> FluentText<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: ToFluentString + Clone> Display for FluentText<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.to_fluent_string())
    }
}
